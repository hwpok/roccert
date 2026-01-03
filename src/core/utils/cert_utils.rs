use anyhow::Result;
use anyhow::{Context, ensure};
use chrono::Utc;
use chrono::{DateTime, TimeZone};
use pem::parse;
use std::path::Path;
use std::{fmt, fs};
use x509_parser::error::X509Error;
use x509_parser::prelude::{GeneralName, X509Certificate};

// 证书信息
#[derive(Debug, Clone)]
pub struct CertInfo {
    pub subject: String,
    pub alternative_name: String,
    pub issuer: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub days_remaining: i64,
    pub is_expired: bool,
}

impl fmt::Display for CertInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time_fmt = "%Y-%m-%d %H:%M UTC";
        writeln!(f, "")?;
        writeln!(f, "--------- Certificate Details ---------")?;
        writeln!(f, "| Subject: {}", self.subject)?;
        writeln!(f, "| Subject Alternative Name: {}", self.alternative_name)?;
        writeln!(f, "| Issuer: {}", self.issuer)?;
        writeln!(f, "| Not Before: {}", self.not_before.format(time_fmt))?;
        writeln!(f, "| Not After:  {}", self.not_after.format(time_fmt))?;
        writeln!(f, "| Days Remaining: {}", self.days_remaining)?;
        writeln!(f, "| Is Expired: {}", self.is_expired)?;
        writeln!(f, "---------------------------------------")
    }
}

// 从指定证书径中获取证书信息
pub fn get_cert_info_path(path: &Path) -> Result<CertInfo> {
    // 检查证书文件是否存在
    ensure!(
        path.exists(),
        "Certificate file not found: {}",
        path.display()
    );
    ensure!(path.is_file(), "Path is not a file: {}", path.display());

    // 读取内容
    let pem_data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    get_info_cert(&pem_data)
}

// 从字符串中获取证书信息
pub fn get_info_cert(pem_data: &str) -> Result<CertInfo> {
    // 转换成Pem
    let pem = parse(&pem_data).context("Failed to parse PEM data")?;
    let (_, cert) = x509_parser::parse_x509_certificate(pem.contents())
        .context("Failed to parse_x509_certificate")?;

    // 提取主题和颁发者
    let subject = cert.subject().to_string();
    let alternative_name =
        extract_san_dns_names(&cert).context("Failed to Gent alternative_name")?;
    let issuer = cert.issuer().to_string();

    // 签发时间
    // 过期时间
    let not_before = Utc
        .timestamp_opt(cert.validity().not_before.timestamp(), 0)
        .single()
        .context("Invalid 'Not After' timestamp in certificate")?;

    // 过期时间
    let not_after = Utc
        .timestamp_opt(cert.validity().not_after.timestamp(), 0)
        .single()
        .context("Invalid 'Not After' timestamp in certificate")?;

    // 计算剩余天数
    let now = Utc::now();
    let duration = not_after.signed_duration_since(now);
    let days_remaining = duration.num_days();

    // 返回剩余天数
    Ok(CertInfo {
        subject,
        alternative_name,
        issuer: issuer,
        not_before,
        not_after,
        days_remaining,
        is_expired: days_remaining <= 0,
    })
}

// 多证书获取被证书保护的域名列表
fn extract_san_dns_names(cert: &X509Certificate) -> Result<String, X509Error> {
    match cert.subject_alternative_name()? {
        Some(ext) => {
            let names: Vec<String> = ext
                .value
                .general_names
                .iter()
                .filter_map(|gn| match gn {
                    GeneralName::DNSName(s) => Some(s.to_string()),
                    _ => None,
                })
                .collect();
            Ok(format!("[{}]", names.join(", ")))
        }
        None => Ok("[]".to_string()),
    }
}

// 保存文件到指定目录
#[allow(unused_variables)]
pub fn backup_and_write_file(
    path: &Path,
    data: &str,
    is_add_perm: bool,
    badkup: bool,
    backup_path: &Path,
) -> Result<(), std::io::Error> {
    // 确保父目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 文件已存在，重命名备份
    if badkup && path.exists() {
        fs::rename(path, &backup_path)?;
        log::info!(
            "backup file: {} -> {}",
            path.display(),
            backup_path.display()
        );
    }

    // 把数据写入文件
    fs::write(path, data)?;
    if is_add_perm {
        // Linux平台下
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test]
    fn test_backup_and_write_file() {
        let result = backup_and_write_file(
            "privkey.pem".as_ref(),
            "data",
            true,
            true,
            "privkey.pem.bak".as_ref(),
        );
        assert!(result.is_ok(), "Fail to save and bakup file")
    }

    #[test_log::test]
    fn test_get_expiry_in_days_from_cert() {
        unsafe {
            std::env::set_var("RUST_BACKTRACE", "full");
            std::env::set_var("ANYHOW", "1");
        }
        let days = get_cert_info_path("fullchain.pem.test".as_ref()).context("huikk");
        if let Err(e) = &days {
            log_user_defined_errors(e);
        }
        assert!(days.is_ok(), "Fail to get expiry_in_days");
        log::info!("days: {}", days.unwrap());
    }
    /// 打印 anyhow::Result 的完整错误链
    pub fn log_user_defined_errors(err: &anyhow::Error) {
        for (i, msg) in err.chain().enumerate() {
            log::error!("Context [{}]: {}", i + 1, msg);
        }
    }
}
