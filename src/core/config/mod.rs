// 配置原生参数模块
pub mod raw_config;

// App运行时参数模块
pub mod app_config;

// 定义枚举
pub mod enums {
    use strum::{Display, EnumString};

    // 证书服务商
    #[derive(Debug, Display, EnumString, Clone)]
    pub enum CaType {
        #[strum(serialize = "letsencrypt")]
        Letsencrypt,
    }

    // 挑战类型
    #[derive(Debug, Display, EnumString, Clone)]
    pub enum ChallengeType {
        #[strum(serialize = "http01")]
        Http01,
        #[strum(serialize = "dns01")]
        Dns01,
    }

    // 证书格式类型
    #[derive(Debug, Display, EnumString, Clone)]
    pub enum CertFomatType {
        #[strum(serialize = "pem")]
        Pem,
    }

    #[derive(Debug, Display, EnumString, Clone)]
    pub enum NoticeLevel {
        Low,
        High,
    }

    impl NoticeLevel {
        pub fn from_tring(level: Option<String>) -> Self {
            match level.as_deref() {
                Some("high") => NoticeLevel::High,
                _ => NoticeLevel::Low,
            }
        }
    }

    // 通知Webhook服务商枚举
    #[derive(Debug, Display, EnumString, Clone)]
    pub enum NoticeWebhookProvider {
        #[strum(serialize = "weico")]
        Wecom, // 企业微信
        #[strum(serialize = "dingtalk")]
        Dingtalk, // 钉钉
        #[strum(serialize = "slack")]
        Slack, // Slack
    }
    // DNS解析服务商
    #[derive(Debug, Display, EnumString, Clone)]
    pub enum DnsProvider {
        #[strum(serialize = "ali")]
        Ali, // 阿里
        #[strum(serialize = "dnsPod")]
        DnsPod, // 腾讯DndPod
    }
}
