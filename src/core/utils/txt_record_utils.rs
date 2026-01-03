use anyhow::{Context, Result, bail};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::time::{Duration, Instant};
use trust_dns_resolver::{
    TokioAsyncResolver as AsyncResolver,
    config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts},
};

// 查找DNS记录是否生效
pub async fn check_txt_record_effective(
    main_domain: &str,
    txt_name: &str,
    txt_value: &str,
    dns_server: &str,
) -> Result<()> {
    // 组合完整的查询域名
    let txt_domain = format!("{}.{}", txt_name, main_domain);

    // 解析DNS服务器地址
    let socket_addr: SocketAddr = ensure_port_53(dns_server)?;

    // 创建解析器配置
    let mut config = ResolverConfig::new();
    config.add_name_server(NameServerConfig {
        socket_addr,
        protocol: Protocol::Udp,
        tls_dns_name: None,
        bind_addr: None,
        trust_negative_responses: false,
    });

    // 创建解析器选项（禁用缓存以确保获取最新记录）
    let mut opts = ResolverOpts::default();
    opts.cache_size = 0;

    // 创建解析器
    let resolver = AsyncResolver::tokio(config, opts);

    // 执行DNS查询
    let lookup = resolver
        .txt_lookup(&txt_domain)
        .await
        .context("Fail to lookup txt-domain")?;

    // 检查所有TXT记录
    let mut found_records = Vec::new();
    for txt_record in lookup.iter() {
        // TXT记录可能由多个数据段组成，需要拼接
        let record_value: String = txt_record
            .txt_data()
            .iter()
            .filter_map(|bytes| std::str::from_utf8(bytes).ok())
            .collect();

        found_records.push(record_value.clone());

        // 如果找到匹配的记录，立即返回成功
        if record_value == txt_value {
            log::debug!("txt record: {}, value: {}", txt_domain, txt_value);
            return Ok(());
        }
    }

    // 没有找到匹配的记录
    bail!(format!(
        "DNS TXT record not found or value does not match\n\
         Domain: {}\n\
         Expected value: {}\n\
         Found records: {:?}",
        txt_domain, txt_value, found_records
    ))
}

// 重试查询
pub async fn check_txt_record_effective_retry(
    main_domain: &str,
    txt_name: &str,
    txt_value: &str,
    dns_server: &str,
    timeout_secs: u64,
    retry_interval_secs: u64,
) -> Result<()> {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let interval = Duration::from_secs(retry_interval_secs);

    let mut attempt = 1;
    while start.elapsed() < timeout {
        log::debug!("DNS check attempt {}...", attempt);
        match check_txt_record_effective(main_domain, txt_name, txt_value, dns_server).await {
            Ok(_) => {
                log::debug!("DNS verification successful on attempt {}", attempt);
                return Ok(());
            }
            Err(e) => {
                if start.elapsed() + interval >= timeout {
                    // 下次重试会超时，直接返回错误
                    bail!(format!(
                        "DNS check failed after {} attempts: {}",
                        attempt, e
                    ));
                }

                log::debug!(
                    "Attempt {} failed: {}. Retrying in {} seconds...",
                    attempt,
                    e,
                    retry_interval_secs
                );

                // 等待后重试
                tokio::time::sleep(interval).await;
                attempt += 1;
            }
        }
    }

    bail!(format!(
        "DNS record did not become effective within {} seconds",
        timeout_secs
    ))
}

/// 获取正确的dns解析服务器的SocketAddr
fn ensure_port_53(addr: &str) -> Result<SocketAddr> {
    // 移除可能的多余空格
    let addr = addr.trim();

    // 已经是完整的SocketAddr
    if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
        return Ok(socket_addr);
    }

    // IPv4地址（如 "8.8.8.8"）
    if let Ok(ipv4) = addr.parse::<Ipv4Addr>() {
        return Ok(SocketAddr::new(IpAddr::V4(ipv4), 53));
    }

    // IPv6地址（如 "2001:4860:4860::8888"）
    if let Ok(ipv6) = addr.parse::<Ipv6Addr>() {
        return Ok(SocketAddr::new(IpAddr::V6(ipv6), 53));
    }

    // 尝试其他格式
    // 可能是主机名(如 "dns.baidu"), 简化处理：直接拼接:53
    if !addr.contains(':') {
        return format!("{}:53", addr)
            .parse::<SocketAddr>()
            .context(format!("Failed to parse '{}:53'", addr));
    }
    bail!("Invalid DNS server address format: '{}'", addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dns_check() {
        // 示例：验证Google的TXT记录
        let result =
            check_txt_record_effective_retry("ch-soft.cn", "hwpok", "123456", "8.8.8.8:53", 300, 2)
                .await;
        assert!(result.is_ok(), "ineffective");
    }
}
