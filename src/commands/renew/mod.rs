pub mod acme;
pub mod challenge;
pub mod dns_provider;

use self::acme::create_certificate_order;
use self::acme::load_or_create_account;
use self::acme::submit_csr_and_down_pem_cert;
use crate::core::config::app_config::Config;
use crate::core::config::enums::NoticeLevel;
use crate::core::notice::send_msg;
use crate::core::utils::cert_utils::*;
use crate::core::utils::executor::ScriptExecutor;
use anyhow::Context;
use anyhow::Result;
use challenge::challenge_order;
use std::path::PathBuf;

// 申请域名证书, 对所有错误进行处理, 禁止传播到上层
pub async fn new_cert(config_file_path: &PathBuf, is_dry_run: bool) {
    // 解析配置
    let config = match Config::new(is_dry_run, config_file_path) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to convert static config to runtime data: {:?}", e);
            return;
        }
    };
    log::info!("Converted static config to runtime data");

    let domains_str = config.target.domains.join(",");

    // 申请新证书
    let result = new_or_renew_cert_raw(&config).await;
    let message = match result {
        Ok(_) => format!(
            "Certificate issued successfully for Domains [{}]",
            domains_str
        ),
        Err(e) => format!(
            "Failed to issue certificate for Domains [{}], {:?}",
            domains_str, e
        ),
    };

    send_msg(&config.notice, &message).await;
}

// 申请证书
pub async fn renew_cert(config_file_path: &PathBuf, is_dry_run: bool) {
    // 解析配置
    let config = match Config::new(is_dry_run, config_file_path) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to convert static config to runtime data: {:?}", e);
            return;
        }
    };
    log::info!("Converted static config to runtime data");

    // 提前计算 domains，避免在 match 分支中重复计算
    let domains_str = config.target.domains.join(",");

    // 获取证书信息并决定结果
    let cert_info_result = get_cert_info_path(&config.target.cert_file)
        .context("Failed to get the remaining validity days of the certificate");

    // 定义等级：0=正常(无需续期), 1=重要(成功/失败/错误)
    let (priority, message) = match cert_info_result {
        Ok(cert_info) => {
            if cert_info.days_remaining > config.target.renew_before_days {
                (
                    0, // Normal
                    format!(
                        "No renewal, certificate of Domains [{}] remaining days: {}.",
                        domains_str, cert_info.days_remaining
                    ),
                )
            } else {
                match new_or_renew_cert_raw(&config).await {
                    Ok(_) => (
                        1, // Important
                        format!(
                            "Renewal successfully, certificate of Domains [{}]",
                            domains_str
                        ),
                    ),
                    Err(e) => (
                        1, // Important
                        format!(
                            "Failed to renewal certificate for Domains [{}], {:?}",
                            domains_str, e
                        ),
                    ),
                }
            }
        }
        Err(e) => (
            1, // Important
            format!(
                "Failed to get the remaining validity days of the certificate, domains: [{}], {:?}",
                domains_str, e
            ),
        ),
    };

    // 只有当事件是重要的，或者用户配置了低级别(接收所有日志)时才发送
    let is_critical_event = priority != 0;
    let is_verbose_mode = matches!(config.notice.level, NoticeLevel::Low);

    if is_critical_event || is_verbose_mode {
        send_msg(&config.notice, &message).await;
    }
}

// 申请或续期域名证书
pub async fn new_or_renew_cert_raw(config: &Config) -> Result<()> {
    // 第1步, 执行开始钩子, 准备环境
    if let Some(command) = config.target.pre_hook.clone() {
        ScriptExecutor::new(10)
            .execute(&command)
            .await
            .context("Fail to excute prehook")?;
        log::info!("Completed executing the start hook");
    }

    // 第2步, 加载或创建用户,
    let account =
        load_or_create_account(&config.ca.account, &config.ca.account_token, config.dry_run)
            .await
            .context("Fail to load or create account")?;
    log::info!("Completed loading or creating account: {}", account.id());

    // 第3步, 创建订单
    let mut order = create_certificate_order(&account, &config.target.domains).await?;
    log::info!("Completed creating order");

    // 第4步, 验证订单域名
    challenge_order(&mut order, &config.challenge).await?;
    log::info!("Completed challenging order");

    // 第5步, 提供证书签名请求和下载证, 返回私钥和证书链
    let privkey_chian = submit_csr_and_down_pem_cert(&config.target.domains, &mut order).await?;
    log::info!("Completed submitting CSR and downing pem cert");

    // 第6步, 处理和保存证书
    let (privkey, cert_chain) = privkey_chian;
    let cert_info = get_info_cert(&cert_chain).context("Failed Getting info from certificate")?;
    log::info!("{}", cert_info);

    let _ = backup_and_write_file(
        &config.target.key_file,
        &privkey,
        true,
        config.target.auto_backup,
        &config.target.key_backup_file,
    )
    .context("Failed to save and back private key file")?;
    let _ = backup_and_write_file(
        &config.target.cert_file,
        &cert_chain,
        false,
        config.target.auto_backup,
        &config.target.cert_backup_file,
    )
    .context("Failed to save and back certificate file")?;
    log::info!("Saved and backed certificate file");

    // 第7步, 执行结束钩子
    if let Some(command) = config.target.post_hook.clone() {
        ScriptExecutor::new(10)
            .execute(&command)
            .await
            .context("Failed to excute prehook")?;
        log::info!("Completed executing the post hook");
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::core::config::app_config;
    use rustls::crypto::{CryptoProvider, ring::default_provider};
    use std::path::PathBuf;

    #[test_log::test(tokio::test)]
    async fn test_request_or_renew_cert() {
        CryptoProvider::install_default(default_provider()).unwrap();
        // 初始化测试环境变量
        dotenvy::dotenv().expect("Failed to initialize environment variables, .env file not found");

        // 从配置文件中读取数据到Config结构体
        let config_path_buf = &PathBuf::from("config.toml");
        let config = app_config::Config::new(true, config_path_buf);
        assert!(
            config.is_ok(),
            "Failed to convert static config to runtime data"
        );

        let config = config.unwrap();
        print!("{:#?}", config);

        // 测试申请或创建证书
        let result = new_or_renew_cert_raw(&config).await;
        if result.is_err() {
            println!("{:?}", result.err());
        }
    }
}
