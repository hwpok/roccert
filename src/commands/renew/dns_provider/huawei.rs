//! # 阿里云 OPEN API 域名管理模块
//!    **未实现**
//!
//! 本模块提供了操作域名TXT记录的核心功能，支持：
//!
//! - 添加 TXT 记录
//! - 删除 TXT 记录
//! - 查询 TXT 记录
//!
//! ## 用途
//! 直接用于上级模块动态分发
//! 接间用于ACME的DNS01挑战验证域名归属
//!
use super::DnsClient;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

// 华为云DNS客户端
#[allow(dead_code)]
#[derive(Debug)]
pub struct HuaWeiDnsClient {
    access_key_id: String,
    access_access_key: String,
    project_id: String,
    endpoint: String,
    http_client: Client,
}

impl HuaWeiDnsClient {
    // 创建华为云客户端
    pub fn new(access_key_id: &str, access_access_key: &str, project_id: &str) -> Self {
        Self {
            access_key_id: access_key_id.to_string(),
            access_access_key: access_access_key.to_string(),
            project_id: project_id.to_string(),
            endpoint: "https://dnspod.tencentcloudapi.com".to_string(),
            http_client: Client::new(),
        }
    }
}

// 实现Dns公共特征
#[async_trait]
impl DnsClient for HuaWeiDnsClient {
    // 添加TXT记录
    async fn add_txt_record(&self, domain: &str, rr: &str, value: &str) -> Result<String> {
        todo!(
            "Adding TXT records to Huawei Cloud DNS is not yet supported, {}, {}, {}",
            domain,
            rr,
            value
        );
    }

    // 删除TXT
    async fn del_txt_record(&self, record_id: &str, _domain: &str) -> Result<String> {
        todo!(
            "Deleting TXT records to Huawei Cloud DNS is not yet supported, {}",
            record_id
        );
    }

    // 查询TXT记录
    async fn qry_txt_records(&self, domain: &str, rr: &str) -> Result<Vec<String>> {
        todo!(
            "Querying TXT records to Huawei Cloud DNS is not yet supported, {}, {}",
            domain,
            rr
        );
    }
}

#[cfg(test)]
mod test {}
