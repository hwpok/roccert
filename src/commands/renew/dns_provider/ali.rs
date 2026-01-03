//! # 阿里云 OPEN API 域名管理模块
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

use self::ali_utils::send_request;
use super::DnsClient;
use super::get_main_domain;
use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use reqwest::Client;
use std::collections::BTreeMap;

// 阿里云DNS客户端
#[derive(Debug)]
pub struct AliDnsClient {
    access_key_id: String,
    access_key_secret: String,
    endpoint: String,
    http_client: Client,
}

impl AliDnsClient {
    // 创建阿里云客户端
    pub fn new(access_key_id: &str, access_key_secret: &str) -> Self {
        Self {
            access_key_id: access_key_id.to_string(),
            access_key_secret: access_key_secret.to_string(),
            endpoint: String::from("https://alidns.aliyuncs.com"),
            http_client: Client::new(),
        }
    }
}

// 实现操作TXT记录特征
#[async_trait]
impl DnsClient for AliDnsClient {
    // 添加TXT记录
    async fn add_txt_record(&self, domain: &str, rr: &str, value: &str) -> Result<String> {
        // 组装请求参数
        let mut params = BTreeMap::new();
        params.insert("Action".to_string(), "AddDomainRecord".to_string());
        params.insert("Version".to_string(), "2015-01-09".to_string());
        params.insert("DomainName".to_string(), get_main_domain(domain)?);
        params.insert("RR".to_string(), rr.to_string());
        params.insert("Type".to_string(), "TXT".to_string());
        params.insert("Value".to_string(), value.to_string());

        // 发送请求
        let resp_xml = send_request(
            &self.access_key_id,
            &self.access_key_secret,
            &self.endpoint,
            params,
            &self.http_client,
        )
        .await
        .context("ALI OPEN API: Failed to add TXT record")?;

        // 解析参数
        if let Some(start) = resp_xml.find("<RecordId>") {
            if let Some(end) = resp_xml.find("</RecordId>") {
                if end > start + 10 {
                    let record_id = &resp_xml[(start + 10)..end];
                    return Ok(record_id.to_string());
                }
            }
        }
        // 解析失败
        bail!(
            "ALI OPEN API: Fail to add TXT record, Response data:  {}",
            resp_xml
        )
    }

    async fn del_txt_record(&self, record_id: &str, _domain: &str) -> Result<String> {
        // 组装请求参数
        let mut params = BTreeMap::new();
        params.insert("Action".to_string(), "DeleteDomainRecord".to_string());
        params.insert("Version".to_string(), "2015-01-09".to_string());
        params.insert("RecordId".to_string(), record_id.to_string());

        // 发送请求
        let resp_xml = send_request(
            &self.access_key_id,
            &self.access_key_secret,
            &self.endpoint,
            params,
            &self.http_client,
        )
        .await
        .context("Fail to delete TXT record")?;
        if resp_xml.contains("<DeleteDomainRecordResponse>") {
            Ok(record_id.to_string())
        } else {
            bail!("Fail to delete TXT record, Response data: {}", resp_xml);
        }
    }

    // 查询txt记录
    async fn qry_txt_records(&self, domain: &str, rr: &str) -> Result<Vec<String>> {
        let mut params = BTreeMap::new();
        params.insert("Action".to_string(), "DescribeDomainRecords".to_string());
        params.insert("Version".to_string(), "2015-01-09".to_string());
        params.insert("DomainName".to_string(), get_main_domain(domain)?);
        params.insert("Type".to_string(), "TXT".to_string());

        // 指定了 RR，就精确过滤
        if !rr.is_empty() {
            params.insert("RRKeyWord".to_string(), rr.to_string());
        }

        // 发送请求
        let resp_xml = send_request(
            &self.access_key_id,
            &self.access_key_secret,
            &self.endpoint,
            params,
            &self.http_client,
        )
        .await
        .context("ALI OPEN API: Fail to query record")?;

        // 没有记录的情况
        if resp_xml.contains("<TotalCount>0</TotalCount>") {
            return Ok(vec![]);
        }

        // 分析记录编号ID
        const START_TAG: &str = "<RecordId>";
        const END_TAG: &str = "</RecordId>";

        let mut ids = Vec::new();
        let mut offset = 0;

        // 循环解析
        while let Some(start) = resp_xml[offset..].find(START_TAG) {
            let abs_start = offset + start;
            let content_start = abs_start + START_TAG.len();

            if let Some(end_rel) = resp_xml[content_start..].find(END_TAG) {
                let abs_end = content_start + end_rel;
                let id_str = &resp_xml[content_start..abs_end];
                ids.push(id_str.trim().to_string());
                // 继续从结束标签之后搜索，避免重复匹配
                offset = abs_end + END_TAG.len();
            } else {
                // 没有闭合标签，跳出
                break;
            }
        }
        Ok(ids)
    }
}

// 阿里云服务加强
mod ali_utils {
    use anyhow::{Result, bail};
    use base64::Engine;
    use chrono::{DateTime, Utc};
    use hmac::{Hmac, Mac};
    use reqwest::Client;
    use sha1::Sha1;
    use std::{
        collections::BTreeMap,
        time::{SystemTime, UNIX_EPOCH},
    };
    // 自实现：URL 编码
    fn percent_encode(s: &str) -> String {
        let mut out = String::new();
        for c in s.bytes() {
            match c {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(c as char);
                }
                b' ' => out.push_str("%20"),
                b'*' => out.push_str("%2A"),
                _ => {
                    out.push('%');
                    out.push_str(&format!("{:02X}", c));
                }
            }
        }
        out
    }

    // 生成 ISO8601 UTC 时间戳
    fn iso8601_utc_now() -> String {
        let now: DateTime<Utc> = Utc::now();
        now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }

    // 随机 Nonce
    fn generate_nonce() -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string()
    }

    // 构建签名字符串
    fn build_signature(params: &BTreeMap<String, String>, secret: &str) -> Result<String> {
        let mut query = String::new();
        for (k, v) in params {
            query.push_str(&format!("&{}={}", percent_encode(k), percent_encode(v)));
        }
        let query = &query[1..];
        let string_to_sign = format!("GET&%2F&{}", percent_encode(query));

        // 生成签名
        let key = format!("{}&", secret);
        let mut mac = Hmac::<Sha1>::new_from_slice(key.as_bytes())?;
        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize();

        // 返回签名数据
        Ok(base64::engine::general_purpose::STANDARD.encode(result.into_bytes()))
    }

    // 发送请求
    pub async fn send_request(
        access_key_id: &str,
        access_key_secret: &str,
        endpoint: &str,
        mut params: BTreeMap<String, String>,
        http_client: &Client,
    ) -> Result<String> {
        // 设置基础参数
        params.insert("AccessKeyId".to_string(), access_key_id.to_string());
        params.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
        params.insert("SignatureVersion".to_string(), "1.0".to_string());
        params.insert("SignatureNonce".to_string(), generate_nonce());
        params.insert("Timestamp".to_string(), iso8601_utc_now());

        // 构建签名
        let signature = build_signature(&params, access_key_secret)?;
        params.insert("Signature".to_string(), signature);

        // 组装请求字符串
        let mut url = format!("{}?", endpoint);
        for (k, v) in &params {
            url.push_str(&format!("{}={}&", percent_encode(k), percent_encode(v)));
        }
        // 删除最后一个"&"
        url.pop();

        // 发送请求
        let resp = http_client.get(&url).send().await?;
        log::debug!("ALI OPEN API: Request data to server: {}", url);

        // 先检查状态
        if !resp.status().is_success() {
            // 请求失败了，读取错误信息并返回
            let resp_text = resp.text().await?;
            bail!("ALI OPEN API: Fail to delete TXT record: {}", resp_text);
        }

        // 返回响应体的内容
        let resp_xml = resp.text().await?;
        log::debug!("ALI OPEN API: Response data from server: {}", resp_xml);
        Ok(resp_xml)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    // 从环境变量获取测试数据
    fn get_creds() -> (String, String) {
        dotenvy::dotenv().expect("Failed to initialize environment variables, .env file not found");
        (
            std::env::var("ALI_ACCESS_KEY_ID").expect("Missing environment variable: ALI_KEY_ID"),
            std::env::var("ALI_ACCESS_KEY_SECRET")
                .expect("Missing environment variable: ALI_SECRET"),
        )
    }

    #[tokio::test]
    async fn test_qry_txt_record() {
        // 获取参数
        let (access_key_id, access_key_secret) = get_creds();

        // 创建阿里云客户端
        let client = AliDnsClient::new(&access_key_id, &access_key_secret);
        let result = client.qry_txt_records("ch-soft.cn", "hwpok").await;
        assert!(result.is_ok(), "Fail to query TXT record");
        let _recorld_id_str = result.unwrap().join(",");
        //assert_eq!(recorld_id_str, "123", "结果不对: {}", recorld_id_str);
    }

    #[tokio::test]
    async fn test_add_del_txt_record() {
        // 获取参数
        let (access_key_id, access_key_secret) = get_creds();
        // 取时间纳秒为RR
        let rr = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string();
        let rr = &format!("hui_{}", rr);

        // 创建阿里云客户端
        let client = AliDnsClient::new(&access_key_id, &access_key_secret);

        // 测试插入
        let result = client.add_txt_record("ch-soft.cn", rr, "adfakiekdkd").await;
        assert!(result.is_ok(), "Fail to add TXT record");

        let record_id = result.unwrap();
        let result = client.del_txt_record(&record_id, "ch-soft.cn").await;
        assert!(result.is_ok(), "Fail to delete TXT record");
    }
}
