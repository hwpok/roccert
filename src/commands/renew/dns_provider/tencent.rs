//! # 腾讯云 DNSPod 域名管理模块
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

use self::tencent_types::{AddRecordResponse, QueryResponse};
use self::tencent_utils::send_request;
use super::DnsClient;
use super::get_main_domain;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

// 腾讯云DNS客户端
#[derive(Debug)]
pub struct TencentDnsClient {
    secret_id: String,
    secret_key: String,
    host: String,
    endpoint: String,
    http_client: Client,
}

impl TencentDnsClient {
    // 创建腾讯云客户端
    pub fn new(secret_id: &str, secret_key: &str) -> Self {
        Self {
            secret_id: secret_id.to_string(),
            secret_key: secret_key.to_string(),
            host: String::from("dnspod.tencentcloudapi.com"),
            endpoint: String::from("https://dnspod.tencentcloudapi.com"),
            http_client: Client::new(),
        }
    }
}

// 实现DnsClient特征
#[async_trait]
impl DnsClient for TencentDnsClient {
    // 添加TXT记录
    async fn add_txt_record(&self, domain: &str, rr: &str, value: &str) -> Result<String> {
        // 获取主域名
        let main_domain = super::get_main_domain(domain)?;

        // 组装参数
        let mut params = HashMap::new();
        params.insert("Domain".to_string(), Value::String(main_domain));
        params.insert("RecordLine".to_string(), Value::String("默认".to_string()));
        params.insert("Value".to_string(), Value::String(value.to_string()));
        params.insert("SubDomain".to_string(), Value::String(rr.to_string()));
        let resp_text = send_request(
            &self.secret_id,
            &self.secret_key,
            &self.host,
            &self.endpoint,
            &self.http_client,
            "CreateTXTRecord",
            params,
        )
        .await
        .context("DNSPod API: Failed to add TXT record")?;

        log::debug!("DNSPod API: Response data: {}", resp_text);

        // 解析 RecordId
        let resp: AddRecordResponse = serde_json::from_str(&resp_text)
            .context("DNSPod API: Failed to parse AddRecordResponse response")?;
        Ok(resp.response.record_id.to_string())
    }

    // 删除TXT
    async fn del_txt_record(&self, record_id: &str, domain: &str) -> Result<String> {
        let record_id: u64 = record_id
            .parse()
            .context("DNSPod API: ailed to convert TXT record ID")?;
        // 组装参数
        let mut params = HashMap::new();
        params.insert("RecordId".to_string(), Value::Number(record_id.into()));
        params.insert("Domain".to_string(), Value::String(domain.to_string()));

        send_request(
            &self.secret_id,
            &self.secret_key,
            &self.host,
            &self.endpoint,
            &self.http_client,
            "DeleteRecord",
            params,
        )
        .await
        .context("DNSPod API: Failed to delete TXT record")?;
        Ok(record_id.to_string())
    }

    // 查询TXT记录
    async fn qry_txt_records(&self, domain: &str, rr: &str) -> Result<Vec<String>> {
        let main_domain = get_main_domain(domain)?;
        let mut params = HashMap::new();
        params.insert("Domain".to_string(), Value::String(main_domain));
        params.insert("RecordType".to_string(), Value::String("TXT".to_string()));

        // 如果指定了子域名（rr），则精确匹配
        if !rr.is_empty() {
            params.insert("Subdomain".to_string(), Value::String(rr.to_string()));
        }

        // 发送请求
        let resp_text = send_request(
            &self.secret_id,
            &self.secret_key,
            &self.host,
            &self.endpoint,
            &self.http_client,
            "DescribeRecordList",
            params,
        )
        .await
        .context("DNSPod API: Failed to query TXT record")?;

        let resp: QueryResponse = serde_json::from_str(&resp_text)
            .context("DNSPod API: Failed to parse QueryResponse response")?;

        // 提取所有 RecordId
        let ids: Vec<String> = resp
            .response
            .record_list
            .into_iter()
            .map(|r| r.record_id.to_string())
            .collect();

        Ok(ids)
    }
}

// 腾讯云OPEN_API所用到的结构体
mod tencent_types {
    use serde::Deserialize;

    // 添加TXT相关结构体
    #[derive(Deserialize)]
    pub struct AddRecordResponse {
        #[serde(rename = "Response")]
        pub response: AddRecordResult,
    }

    #[derive(Deserialize)]
    pub struct AddRecordResult {
        #[serde(rename = "RecordId")]
        pub record_id: u64,
    }

    // 查询TXT相关结构体
    #[derive(Deserialize)]
    pub struct QueryResponse {
        #[serde(rename = "Response")]
        pub response: RecordListResponse,
    }

    #[derive(Deserialize)]
    pub struct RecordListResponse {
        #[serde(rename = "RecordList")]
        pub record_list: Vec<RecordList>,
    }

    #[derive(Deserialize)]
    pub struct RecordList {
        #[serde(rename = "RecordId")]
        pub record_id: u64,
    }
}

// 工具函数
pub mod tencent_utils {
    use anyhow::{Result, bail};
    use chrono::Utc;
    use hex;
    use hmac::{Hmac, Mac};
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;
    type HmacSha256 = Hmac<sha2::Sha256>;
    use reqwest::Client;

    // 获取当前UTC时间戳
    fn timestamp_now() -> i64 {
        Utc::now().timestamp()
    }

    // 获取日期字符串 YYYY-MM-DD
    fn date_string() -> String {
        Utc::now().date_naive().format("%Y-%m-%d").to_string()
    }

    // HMAC-SHA256 签名工具函数
    fn hmac_sha256(key: &[u8], msg: &str) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key length is valid");
        mac.update(msg.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }

    // 构造签名（TC3-HMAC-SHA256）
    fn build_authorization(
        secret_id: &str,
        secret_key: &str,
        host: &str,
        payload: &str,
        timestamp: i64,
        date_str: &str,
    ) -> Result<String> {
        let service = "dnspod";
        let credential_scope = format!("{}/{}/tc3_request", date_str, service);

        // 计算 SecretDate/SecretService/SecretSigning
        let secret_date = hmac_sha256(format!("TC3{}", secret_key).as_bytes(), date_str);
        let secret_service = hmac_sha256(&secret_date, service);
        let secret_signing = hmac_sha256(&secret_service, "tc3_request");

        // 构造规范请求（CanonicalRequest）
        let hashed_payload = hex::encode(Sha256::digest(payload.as_bytes()));
        let canonical_request = format!(
            "POST\n/\n\ncontent-type:application/json; charset=utf-8\nhost:{}\n\ncontent-type;host\n{}",
            host, hashed_payload
        );

        // 构造待签名字符串（StringToSign）
        let string_to_sign = format!(
            "TC3-HMAC-SHA256\n{}\n{}\n{}",
            timestamp,
            credential_scope,
            hex::encode(Sha256::digest(canonical_request))
        );

        // 计算最终签名
        let signature = hex::encode(hmac_sha256(&secret_signing, &string_to_sign));

        // 拼接 Authorization 头
        Ok(format!(
            "TC3-HMAC-SHA256 Credential={}/{}, SignedHeaders=content-type;host, Signature={}",
            secret_id, credential_scope, signature
        ))
    }

    // 发送 API 请求
    pub async fn send_request(
        secret_id: &str,
        secret_key: &str,
        host: &str,
        endpoint: &str,
        http_client: &Client,
        action: &str,
        params: HashMap<String, Value>,
    ) -> Result<String> {
        let timestamp = timestamp_now();
        let date_str = date_string();

        let payload = serde_json::to_string(&params)?;
        let authorization =
            build_authorization(secret_id, secret_key, host, &payload, timestamp, &date_str)?;

        // 构造 headers
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json; charset=utf-8".parse()?,
        );
        headers.insert("Host", host.parse()?);
        headers.insert("X-TC-Action", action.parse()?);
        headers.insert("X-TC-Version", "2021-03-23".parse()?);
        headers.insert("X-TC-Timestamp", timestamp.to_string().parse()?);
        headers.insert("Authorization", authorization.parse()?);

        // 发送请求
        let resp = http_client
            .post(endpoint)
            .headers(headers)
            .body(payload)
            .send()
            .await?;

        let status = resp.status();
        let resp_text = resp.text().await?;

        // 检查请求状态
        if !status.is_success() {
            bail!("DNSPod API: request failed({}): {}", status, resp_text);
        }
        log::debug!("DNSPod API: Response data: {}", resp_text);

        // 从报文件初步判断是请求成功还是失败
        if resp_text.contains("Error") {
            bail!("DNSPod API: Fail to delete TXT record: {}", resp_text);
        }
        Ok(resp_text)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // 从环境变量获取测试数据
    fn get_creds() -> (String, String) {
        dotenvy::dotenv().expect("Failed to initialize environment variables, .env file not found");
        (
            std::env::var("WX_SECRET_ID").expect("Missing environment variable: WX_SECRET_ID"),
            std::env::var("WX_SECRET_KEY").expect("Missing environment variable: WX_SECRET_KEY"),
        )
    }

    #[tokio::test]
    async fn test_add_txt_record() {
        let (secret_id, secret_key) = get_creds();
        println!("id: {}, key: {}", secret_id, secret_key);
        let tecent_clent = TencentDnsClient::new(&secret_id, &secret_key);
        let result = tecent_clent
            .add_txt_record("aosunlive.com", "tang", "123")
            .await;
        assert!(result.is_ok(), "Failed to add TXT record");
        println!("Data: {:?}", result.unwrap());
    }

    #[tokio::test]
    async fn test_del_txt_record() {
        let (secret_id, secret_key) = get_creds();
        println!("id: {}, key: {}", secret_id, secret_key);
        let tecent_clent = TencentDnsClient::new(&secret_id, &secret_key);
        let result = tecent_clent
            .del_txt_record("2226313807", "aosunlive.com")
            .await;
        assert!(result.is_ok(), "Failed to delete TXT record");
        println!("Data: {:?}", result);
    }

    #[tokio::test]
    async fn test_qry_txt_records() {
        let (secret_id, secret_key) = get_creds();
        println!("id: {}, key: {}", secret_id, secret_key);
        let tecent_clent = TencentDnsClient::new(&secret_id, &secret_key);
        let result = tecent_clent.qry_txt_records("aosunlive.com", "wang").await;
        assert!(result.is_ok(), "Failed to query TXT record");
        println!("Data: {:?}", result);
    }
}
