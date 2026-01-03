use crate::core::utils::env_var_utils::*;
use crate::core::utils::file_utils::append_suffix_to_filename;
use crate::core::utils::file_utils::get_fix_path_buf;
use crate::core::utils::validator::*;

use super::enums::NoticeWebhookProvider;
use super::enums::*;
use super::raw_config::Ca as RawCa;
use super::raw_config::Challenge as RawChallenge;
use super::raw_config::ChallengeDns01 as RawChallengeDns01;
use super::raw_config::ChallengeHttp01 as RawChallengeHttp01;
use super::raw_config::Config as RawConfig;
use super::raw_config::Notice as RawNotice;
use super::raw_config::NoticeEmail as RawNoticeEmail;
use super::raw_config::NoticeWebhook as RawNoticeWebhook;
use super::raw_config::Target as RawTarget;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use chrono::Utc;
use log::LevelFilter;
use std::fs;
use std::path::PathBuf;

// 参数结构体
#[derive(Debug, Clone)]
pub struct Config {
    pub dry_run: bool,
    pub target: Target,
    pub ca: Ca,
    pub challenge: Challenge,
    pub notice: Notice,
}

impl Config {
    pub fn new(dry_run: bool, config_path: &PathBuf) -> Result<Self> {
        // 读取文件内容
        let content = fs::read_to_string(&config_path).context("Failed to read config file")?;

        // 解析 TOML 到配置对象
        let raw_config =
            toml::from_str::<RawConfig>(&content).context("Fail to read config file")?;

        // 目标相关
        let target = raw_config
            .target
            .map(|t| Target::try_new(t, dry_run))
            .transpose()?
            .context("Invalid target")?;

        // 证书相关
        let ca = raw_config
            .ca
            .map(|c| Ca::try_new(c, dry_run))
            .transpose()?
            .context("Invalid certificate")?;

        // 挑战相关
        let challenge = raw_config
            .challenge
            .map(|c| Challenge::try_new(c))
            .transpose()?
            .context("Invalid challenge")?;

        // 通知相关, 把Notice解包, 以名后续的复杂处理
        let notice = raw_config
            .notice
            .map(|n| Notice::try_new(n))
            .transpose()?
            .context("Invalid challenge")
            .unwrap_or_default();

        Ok(Self {
            dry_run,
            target,
            ca,
            challenge,
            notice,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Target {
    pub domains: Vec<String>,
    pub format: CertFomatType,
    pub cert_file: PathBuf,
    pub key_file: PathBuf,
    pub cert_backup_file: PathBuf,
    pub key_backup_file: PathBuf,
    pub renew_before_days: i64,
    pub auto_backup: bool,
    pub pre_hook: Option<String>,
    pub post_hook: Option<String>,
    pub log_level: LevelFilter,
}

impl Target {
    fn try_new(raw: RawTarget, dry_run: bool) -> Result<Self> {
        let domains = raw.domains.context("Missing domains")?;
        let domains: Vec<String> = domains
            .into_iter()
            .filter(|domain| is_domain(domain))
            .collect();
        ensure!(!domains.is_empty(), "Domains is empty");

        // 处理证书格式
        let format = raw.format.unwrap_or("pem".to_string());
        let format = format
            .parse::<CertFomatType>()
            .context("format can be pem")?;

        // 处理证书目录默认值
        let cert_file = get_fix_path_buf(raw.cert_file, "test", dry_run, "fullchain.pem")
            .context("Fail to operate account token fiel")?;

        // 处理私钥默认值
        let key_file = get_fix_path_buf(raw.key_file, "test", dry_run, "privkey.pem")
            .context("Fail to operate account token fiel")?;

        // 生成时间戳
        let timestamp = {
            let now = Utc::now();
            format!(
                ".{}_{:03}",
                now.format("%Y%m%d_%H%M%S"),
                now.timestamp_subsec_millis()
            )
        };

        // 处理备份文件路径
        let cert_backup_file = append_suffix_to_filename(&cert_file.clone(), &timestamp)?;
        let key_backup_file = append_suffix_to_filename(&key_file.clone(), &timestamp)?;

        let auto_backup = raw.auto_backup.unwrap_or(true);
        let renew_before_days = raw.renew_before_days.unwrap_or(30i64);
        let renew_before_days = match renew_before_days {
            val @ 10..=30 => val,
            _ => 30,
        };

        let pre_hook = raw.pre_hook;
        let post_hook = raw.post_hook;
        let log_level = raw.log_level.unwrap_or("error".to_string());

        let log_level = log_level
            .parse::<LevelFilter>()
            .context("Invalid log levle")?;
        Ok(Self {
            domains,
            format,
            cert_file,
            key_file,
            cert_backup_file,
            key_backup_file,
            renew_before_days,
            auto_backup,
            pre_hook,
            post_hook,
            log_level,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Ca {
    pub r#type: CaType,
    pub account: String,
    pub account_token: PathBuf,
}

impl Ca {
    fn try_new(raw: RawCa, dry_run: bool) -> Result<Self> {
        // 证书颁发机构的类型
        let r#type = raw.r#type.unwrap_or("letsencrypt".to_string());
        let r#type = r#type
            .parse::<CaType>()
            .context("Ca type can be letsencrypt")?;

        // 账号
        let account = raw.account.context("Miss account")?;
        ensure!(is_email(&account), "Invalid account email");

        // 处理原始值
        let account_token =
            get_fix_path_buf(raw.account_token, "test", dry_run, "letsencrypt_token.json")
                .context("Fail to operate account token fiel")?;

        Ok(Self {
            r#type,
            account,
            account_token,
        })
    }
}

// 挑战配置
#[derive(Debug, Clone)]
pub struct Challenge {
    pub r#type: ChallengeType,
    pub http01: Option<ChallengeHttp01>,
    pub dns01: Option<ChallengeDns01>,
}

impl Challenge {
    fn try_new(raw: RawChallenge) -> Result<Self> {
        // 挑战种类
        let r#type = raw.r#type.unwrap_or("http01".to_string());
        let r#type = r#type
            .parse::<ChallengeType>()
            .context("challenge type is invalid")?;

        // 根据挑战各类检查
        let (http01, dns01) = match r#type {
            ChallengeType::Http01 => (raw.http01.map(ChallengeHttp01::try_new).transpose()?, None),
            ChallengeType::Dns01 => (None, raw.dns01.map(ChallengeDns01::try_new).transpose()?),
        };

        Ok(Self {
            r#type,
            http01,
            dns01,
        })
    }
}

// Http01配置
#[derive(Debug, Clone)]
pub struct ChallengeHttp01 {
    pub webroot: PathBuf,
}

impl ChallengeHttp01 {
    fn try_new(raw: RawChallengeHttp01) -> Result<Self> {
        let webroot = raw.webroot.context("Missing webroot")?;
        ensure!(is_local_dir(&webroot), "Webroot is not local dir");
        Ok(Self {
            webroot: PathBuf::from(&webroot),
        })
    }
}

// Dns配置
#[derive(Debug, Clone)]
pub struct ChallengeDns01 {
    pub provider: DnsProvider,
    pub access_key_id: String,
    pub access_key_secret: String,
    pub extra_param1: Option<String>,
    pub check_resolver: String,
}

impl ChallengeDns01 {
    fn try_new(raw: RawChallengeDns01) -> Result<Self> {
        let provider = raw.provider.context("Missing provider")?;
        let provider = provider
            .parse::<DnsProvider>()
            .context("Unsupported or invalid DNS provider")?;
        let access_key_id = raw.access_key_id.context("Missing access_key_id")?;
        let access_key_id =
            get_env_var_name_value(&access_key_id).context("Missing access_key_id")?;
        let access_key_secret = raw.access_key_secret.context("Missing access_key_secret")?;
        let access_key_secret =
            get_env_var_name_value(&access_key_secret).context("Missing access_key_secret")?;
        let extra_param1 = Some(raw.extra_param1.unwrap_or_default());
        let check_resolver = raw.check_resolver.unwrap_or("8.8.8.8".to_string());

        // Dns解析商不为空, 需要看是否是IP或域名
        ensure!(
            (check_resolver.is_empty() || is_domain(&check_resolver) || is_ip(&check_resolver)),
            "check_resolver must be an empty string, a valid IP, or a valid domain"
        );

        Ok(Self {
            provider,
            access_key_id,
            access_key_secret,
            extra_param1,
            check_resolver,
        })
    }
}

// 通知信息配置
#[derive(Debug, Clone)]
pub struct Notice {
    pub enabled: bool,
    pub level: NoticeLevel,
    pub webhook_notice: Option<NoticeWebhook>,
    pub email_notice: Option<NoticeEmail>,
}

impl Default for Notice {
    fn default() -> Self {
        Self {
            enabled: false,
            level: NoticeLevel::High,
            webhook_notice: None,
            email_notice: None,
        }
    }
}
impl Notice {
    fn try_new(raw: RawNotice) -> Result<Self> {
        let enabled = raw.enabled.unwrap_or(false);
        let level = NoticeLevel::from_tring(raw.level);

        // 可以不配置NoticeWebhook和NoticeEmail, 但是不可配错
        let (webhook_notice, email_notice) = if enabled {
            (
                raw.webhook.map(NoticeWebhook::try_new).transpose()?,
                raw.email.map(NoticeEmail::try_new).transpose()?,
            )
        } else {
            (None, None)
        };

        Ok(Self {
            enabled,
            level,
            webhook_notice,
            email_notice,
        })
    }
}
#[derive(Debug, Clone)]
pub struct NoticeWebhook {
    pub provider: NoticeWebhookProvider,
    pub url: String,
    pub extra_param1: Option<String>,
}

impl NoticeWebhook {
    fn try_new(raw: RawNoticeWebhook) -> Result<Self> {
        // 检查服务商
        let provider = raw.provider.context("Missing provider")?;
        let provider = provider
            .parse::<NoticeWebhookProvider>()
            .context("Unsupported provider. Use 'weixin' or 'dingtalk'.")?;

        // 检查URL
        let url = raw.url.context("Missing url")?;
        let url = get_env_var_name_value(&url).context("Missing url")?;
        ensure!(is_url(&url), "Invalid url");

        // 返回自已
        Ok(Self {
            provider,
            url,
            extra_param1: raw.extra_param1,
        })
    }
}

// 通知Email参数
#[derive(Debug, Clone)]
pub struct NoticeEmail {
    pub smtp: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    pub template_file: Option<String>,
}
impl NoticeEmail {
    fn try_new(raw: RawNoticeEmail) -> Result<Self> {
        let smtp = raw.smtp.context("Missing smtp")?;
        ensure!(is_domain(&smtp), "Invalid smtp");
        let smtp_port = raw.smtp_port.unwrap_or(465);
        let username = raw.username.context("Missing username")?;
        let username = get_env_var_name_value(&username).context("Missing username")?;
        let password = raw.password.context("Missing password")?;
        let password = get_env_var_name_value(&password).context("Missing password")?;
        let from = raw.from.context("Missing from")?;
        ensure!(is_email(&from), "Invalid email format for 'from'");
        let raw_to = raw.to.context("Missing to")?;
        let emails: Vec<String> = raw_to.into_iter().filter(|email| is_email(email)).collect();
        ensure!(!emails.is_empty(), "Invalid email format for 'to'");
        let subject = raw
            .subject
            .unwrap_or("Certificate Auto-Renewal Notification".to_string());

        Ok(Self {
            smtp,
            smtp_port,
            username,
            password,
            from,
            to: emails,
            subject,
            template_file: None,
        })
    }
}

#[cfg(test)]
mod config_test {
    use std::path::PathBuf;

    #[test]
    fn test_config() {
        dotenvy::dotenv().expect("Failed to initialize environment variables, .env file not found");
        let config_path_buf = PathBuf::from("config.toml");
        let config = super::Config::new(true, &config_path_buf);
        if let Err(e) = config {
            println!("{:?}", e);
        } else {
            print!("{:#?}", config.unwrap());
        }
        /*
        assert!(
            config.is_ok(),
            "Failed to convert static config to runtime data"
        );
         */
    }
}
