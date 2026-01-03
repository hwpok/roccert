use serde::Deserialize;

// 参数结构体
#[derive(Debug, Deserialize)]
pub struct Config {
    pub target: Option<Target>,
    pub ca: Option<Ca>,
    pub challenge: Option<Challenge>,
    pub notice: Option<Notice>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Target {
    pub domains: Option<Vec<String>>,
    pub format: Option<String>,
    pub cert_file: Option<String>,
    pub key_file: Option<String>,
    pub renew_before_days: Option<i64>,
    pub auto_backup: Option<bool>,
    pub pre_hook: Option<String>,
    pub post_hook: Option<String>,
    pub log_level: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Ca {
    pub r#type: Option<String>,
    pub account: Option<String>,
    pub account_token: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Challenge {
    pub r#type: Option<String>,
    pub http01: Option<ChallengeHttp01>,
    pub dns01: Option<ChallengeDns01>,
}

#[derive(Debug, Deserialize)]
pub struct ChallengeHttp01 {
    pub webroot: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChallengeDns01 {
    pub provider: Option<String>,
    pub access_key_id: Option<String>,
    pub access_key_secret: Option<String>,
    pub extra_param1: Option<String>,
    pub check_resolver: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Notice {
    pub enabled: Option<bool>,
    pub level: Option<String>,
    pub webhook: Option<NoticeWebhook>,
    pub email: Option<NoticeEmail>,
}

#[derive(Debug, Deserialize)]
pub struct NoticeWebhook {
    pub provider: Option<String>,
    pub url: Option<String>,
    pub extra_param1: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NoticeEmail {
    pub smtp: Option<String>,
    pub smtp_port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from: Option<String>,
    pub to: Option<Vec<String>>,
    pub subject: Option<String>,
    pub template_file: Option<String>,
}

#[cfg(test)]
mod raw_config_tests {
    use super::Config;
    use std::fs;

    // 测试对原始数据的转换
    #[test]
    fn test_parse_raw_config() {
        // 读取环境变量
        dotenvy::dotenv().expect("Failed to initialize environment variables, .env file not found");

        // 读取文件内容
        let content = fs::read_to_string("config.toml");
        assert!(content.is_ok(), "Fail to read config.xml");

        // 解析 TOML
        let config = toml::from_str::<Config>(&content.unwrap());
        assert!(config.is_ok(), "Fail to parse config.xml");
        print!("{:#?}", config);
    }
}
