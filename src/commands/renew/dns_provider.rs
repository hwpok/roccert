use crate::core::config::enums::DnsProvider;

use self::ali::AliDnsClient;
use self::tencent::TencentDnsClient;
use anyhow::{Context, Result};
use async_trait::async_trait;

pub mod ali;
pub mod huawei;
pub mod tencent;

// Dns服务商接口
#[async_trait]
pub trait DnsClient {
    // 查询txt记录
    async fn qry_txt_records(&self, domain: &str, rr: &str) -> Result<Vec<String>>;
    // 添加txt记录
    async fn add_txt_record(&self, domain: &str, rr: &str, value: &str) -> Result<String>;
    // 删除txt记录
    async fn del_txt_record(&self, record_id: &str, domain: &str) -> Result<String>;
}

// 配置结构
#[derive(Debug)]
pub struct DnsClientConfig {
    pub dns_provider: DnsProvider,
    pub secret_id: String,
    pub secret_key: String,
    pub extra: Option<String>,
}

impl DnsClientConfig {
    pub fn new(
        dns_provider: DnsProvider,
        secret_id: &str,
        secret_key: &str,
        extra: Option<String>,
    ) -> Result<Self> {
        Ok(Self {
            dns_provider,
            secret_id: secret_id.to_string(),
            secret_key: secret_key.to_string(),
            extra,
        })
    }
}

// DNS客户端工厂
pub struct DnsClientFactory;

impl DnsClientFactory {
    // 动态分发特征
    pub fn create(config: &DnsClientConfig) -> Box<dyn DnsClient> {
        match config.dns_provider {
            DnsProvider::DnsPod => {
                Box::new(TencentDnsClient::new(&config.secret_id, &config.secret_key))
            }
            DnsProvider::Ali => Box::new(AliDnsClient::new(&config.secret_id, &config.secret_key)),
        }
    }
}

// 从子域名中获取主域名
pub fn get_main_domain(domain: &str) -> Result<String> {
    let domain = psl::domain_str(domain).context("转换域名失败")?;
    Ok(domain.to_string())
}

// 测试代码
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::enums::DnsProvider;

    // 测试获取主域名
    #[test]
    fn test_get_main_domain() {
        let result = get_main_domain("p.ch-soft.com.cn");
        assert!(result.is_ok(), "获取主域名应成功，但返回错误: {:?}", result);
        assert_eq!(result.unwrap(), "ch-soft.com.cn", "获取主域名失败");
    }

    #[tokio::test]
    async fn test_dns_client_factory() {
        // 从环境变量中拿参数
        let (secret_id, secret_key) = (
            &std::env::var("ALI_ACCESS_KEY_ID").expect("请设置环境变量 ALI_KEY_ID"),
            &std::env::var("ALI_ACCESS_KEY_SECRET").expect("请设置环境变量 ALI_SECRET"),
        );

        let dns_provider = "ali".parse::<DnsProvider>().unwrap();
        // 创建配置参数
        let config = DnsClientConfig::new(dns_provider, secret_id, secret_key, None);
        assert!(
            config.is_ok(),
            "创建参数对象失败, {:?}",
            config.unwrap_err()
        );

        // 获取对应的客户端
        let client = DnsClientFactory::create(&config.unwrap());
        let result = client.qry_txt_records("ch-soft.cn", "abc123").await;
        assert!(
            result.is_ok(),
            "查询域名TXT记录失败: {}",
            result.unwrap_err()
        );
    }
}
