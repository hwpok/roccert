use crate::commands::renew::dns_provider::DnsClientConfig;
use crate::core::config::app_config::Challenge;
use crate::core::config::enums::ChallengeType;
use anyhow::{Context, Result};
use instant_acme::Order;

/// 挑战验证域名
pub async fn challenge_order(order: &mut Order, challenge: &Challenge) -> Result<()> {
    match challenge.r#type {
        // http01挑战
        ChallengeType::Http01 => {
            let http01 = challenge.clone().http01.context("http01 error")?;
            challenge::challenge_domain_http01(order, http01.webroot).await?
        }
        // Dns01挑战
        ChallengeType::Dns01 => {
            let dns01 = challenge.dns01.clone().context("dns01 error")?;
            let dns_client_config = DnsClientConfig {
                dns_provider: dns01.provider,
                secret_id: dns01.access_key_id.clone(),
                secret_key: dns01.access_key_secret.clone(),
                extra: dns01.extra_param1,
            };
            challenge::challenge_domain_dns01(order, &dns_client_config, &dns01.check_resolver)
                .await?;
        }
    }
    Ok(())
}

mod challenge {
    use super::super::dns_provider::{DnsClientConfig, DnsClientFactory, get_main_domain};
    use crate::core::utils::txt_record_utils;
    use anyhow::{Result, bail};
    use instant_acme::Order;
    use instant_acme::{AuthorizationStatus, ChallengeType, RetryPolicy};
    use reqwest::Client;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::Duration;
    use tokio::time::{sleep, timeout};

    /// Http01的方式挑战域名
    pub async fn challenge_domain_http01(order: &mut Order, webroot: PathBuf) -> Result<()> {
        log::debug!("Starting challenge domain");
        let mut challenge_files = Vec::new();
        let mut authorizations = order.authorizations();
        while let Some(authz_handle) = authorizations.next().await {
            let mut authz_handle = authz_handle?;
            // 已经验证时, 不需要再次验证
            if authz_handle.status == AuthorizationStatus::Valid {
                continue;
            }

            // 选择 Http01 Challenge
            let mut challenge = authz_handle
                .challenge(ChallengeType::Http01)
                .ok_or_else(|| anyhow::anyhow!("Missing challenge type: Http01"))?;

            // 获取文件名和文件内容
            let token = &challenge.token;
            let key_auth = challenge.key_authorization();
            let dns_value = key_auth.as_str();

            // 向指定目录写入文件和文件内容
            let challenge_file_path = webroot.join(token);
            fs::write(&challenge_file_path, dns_value)?;
            log::debug!(
                "Challenge file has been created: {}",
                challenge_file_path.display()
            );
            challenge_files.push(challenge_file_path);
            // 获取当前域名, 自测挑战文件是否生效, 最长5分钟, 适应非服务上运本CLI的场景
            let domain = challenge.identifier().to_string();
            if !self_check_challenge(&domain, &token, &dns_value).await {
                log::error!("Failed to Self-Checking for domain: {}", domain);
                bail!("Failed to Self-Checking for domain: {}", domain);
            }

            // 自检通过后, 再通知Acme验证域名
            challenge.set_ready().await?;
        }

        // 轮询直到 Ready
        log::debug!("Polling challenge status...");
        // 自定义策略：初始延迟2秒，最大超时5分钟
        let retry_policy = RetryPolicy::new()
            .initial_delay(Duration::from_secs(2))
            .timeout(Duration::from_secs(300));

        // 轮询域名验证是否生效
        let status = order.poll_ready(&retry_policy).await;

        // 清理掉生成的挑战文件
        for challenge_file_path in challenge_files {
            match fs::remove_file(challenge_file_path) {
                Ok(_) => log::info!("Delete challenge file successfully"),
                Err(e) => log::warn!("Failed to Delete challenge file, {:?}", e),
            };
        }

        // 处理单订单状态
        match status {
            // 验证成功
            Ok(instant_acme::OrderStatus::Ready) => {
                log::debug!("Domain challenge successful, Ready");
                return Ok(());
            }
            // 验证失败
            Ok(instant_acme::OrderStatus::Invalid) => {
                log::debug!("Fail to challenge Domain, Invalid");
                return Err(anyhow::anyhow!("Fail to challenge Domain, Invalid"));
            }
            // 其它错误
            _other => Err(anyhow::anyhow!(
                "Challenge validation failed. The status is: Invalid"
            )),
        }
    }

    // 自测试挑战文件是否生效
    pub async fn self_check_challenge(domain: &str, token: &str, dns_value: &str) -> bool {
        // 拼接访问URL
        let full_url = format!("http://{}/.well-known/acme-challenge/{}", domain, token);
        log::info!("Self-checking url: {}", full_url);

        // 设定总超时时间为5分钟
        let total_timeout_duration = Duration::from_secs(5 * 60);

        // 启动超时控制
        let result = timeout(total_timeout_duration, async {
            // 创建Client，设置单个请求的连接超时为3秒，避免重试时卡死太久
            let client = Client::builder()
                .connect_timeout(Duration::from_secs(3))
                .build();

            // 创建Client
            if client.is_err() {
                log::error!(
                    "Failed to create client for self-checking, {:?}",
                    client.err()
                );
                return false;
            }

            let client = client.unwrap();

            loop {
                // 尝试发送请求
                match client.get(&full_url).send().await {
                    Ok(response) => {
                        // 返回的状态为200,
                        if response.status() == reqwest::StatusCode::OK {
                            match response.text().await {
                                Ok(content) => {
                                    // 验证相等性
                                    log::info!("Self-Checking, status: {}", content == dns_value);
                                    return content == dns_value;
                                }
                                Err(e) => {
                                    // 读取Body失败(可能是连接中断),继续重试
                                    log::error!("Failed to read response body: {:?}", e);
                                    continue;
                                }
                            }
                        } else {
                            log::debug!("Self-checking failed with status: {}", response.status());
                        }
                    }
                    Err(e) => {
                        log::warn!("Self-check request error: {}", e);
                    }
                }

                // 每3秒重试一次
                sleep(Duration::from_secs(3)).await;
            }
        })
        .await;
        result.unwrap_or(false)
    }

    // DNS01的方式挑战域名
    pub async fn challenge_domain_dns01(
        order: &mut Order,
        dns_client_config: &DnsClientConfig,
        check_resolver: &str,
    ) -> Result<()> {
        log::debug!("Starting challenge domain");
        let mut txt_record_id_map = HashMap::new();
        let dns_client = DnsClientFactory::create(dns_client_config);
        let mut authorizations = order.authorizations();
        while let Some(authz_handle) = authorizations.next().await {
            let mut authz_handle = authz_handle?;
            // 已经验证时, 不需要再次验证
            if authz_handle.status == AuthorizationStatus::Valid {
                continue;
            }

            // 选择 DNS Challenge
            let mut challenge = authz_handle
                .challenge(ChallengeType::Dns01)
                .ok_or_else(|| anyhow::anyhow!("Missing challenge type: DNS01"))?;

            // 构建所需TXT记录参数
            let record_value = challenge.key_authorization().dns_value();
            let domain = challenge.identifier().to_string();
            let main_domain = get_main_domain(&domain)?;
            let record_name = {
                if domain == main_domain || domain == format!("*.{}", main_domain) {
                    "_acme-challenge".to_string()
                } else {
                    format!(
                        "_acme-challenge.{}",
                        &domain[..domain.len() - main_domain.len() - 1]
                    )
                }
            };

            // TXT已添加过, 结束本次循环
            if txt_record_id_map.contains_key(&record_name) {
                continue;
            }

            // 打印信息
            log::debug!(
                "challenge domain: {}, add TXT record: rr: {},  value: {}",
                domain,
                record_name,
                record_value
            );

            // 查找记录是否已存, 已存在就先删除再加
            let exist_record_vec = dns_client
                .qry_txt_records(&main_domain, &record_name)
                .await?;

            // 循环删除已存在的解析
            for exist_record_id in &exist_record_vec {
                if let Ok(_) = dns_client
                    .del_txt_record(exist_record_id, main_domain.as_str())
                    .await
                {
                    log::debug!(
                        "Duplicate records deleted successfully: {}",
                        exist_record_id
                    );
                }
            }

            // 调用API, 添加TXT记录
            let record_id = dns_client
                .add_txt_record(&main_domain, &record_name, &record_value)
                .await?;

            log::debug!(
                "Add TXT record successly, the record identifier is: {}",
                record_id
            );
            txt_record_id_map.insert(
                record_name.clone(),
                (record_id.clone(), main_domain.clone()),
            );
            // 检查域名的TXT记录是否生效
            let check_result = txt_record_utils::check_txt_record_effective_retry(
                &main_domain,
                &record_name,
                &record_value,
                check_resolver,
                300,
                2,
            )
            .await;
            if check_result.is_ok() {
                log::debug!("TXT record is active: {}", record_name);
            }

            // 通知验证
            challenge.set_ready().await?;
        }

        // 轮询直到 Ready
        log::debug!("Polling challenge status...");
        // 自定义策略：初始延迟 2 秒，最大超时 5 分钟
        let retry_policy = RetryPolicy::new()
            .initial_delay(Duration::from_secs(2))
            .timeout(Duration::from_secs(300));

        // 检查域名验证是否生效
        let status = order.poll_ready(&retry_policy).await;

        // 循环删除本次验证生的的TXT记录
        for (_, pair) in &txt_record_id_map {
            let (record_id, main_domain) = pair;
            match dns_client.del_txt_record(record_id, main_domain).await {
                Ok(_) => {
                    log::debug!("Successfully delete TXT record: {}", pair.0.as_str());
                }
                Err(e) => {
                    log::debug!("Failed to delete TXT record: {:?}", e);
                }
            }
        }

        // 处理单订单状态
        match status {
            // 验证成功
            Ok(instant_acme::OrderStatus::Ready) => {
                log::debug!("Domain challenge successful, Ready");
                return Ok(());
            }
            // 验证失败
            Ok(instant_acme::OrderStatus::Invalid) => {
                log::debug!("Fail to challenge Domain, Invalid");
                return Err(anyhow::anyhow!("Fail to challenge Domain, Invalid"));
            }
            // 其它错误
            _other => Err(anyhow::anyhow!(
                "Challenge validation failed. The status is: Invalid"
            )),
        }
    }
}
