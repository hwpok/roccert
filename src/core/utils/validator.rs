use email_address::EmailAddress;
use std::path::Path;
use url::Url;

// 关断是否是本地的文件夹
pub fn is_local_dir(dir: &str) -> bool {
    Path::new(dir).is_dir()
}

// 检查域名是否合法
pub fn is_domain(domain: &str) -> bool {
    EmailAddress::is_valid_domain(domain)
}

// 检查email是否合法
pub fn is_email(email: &str) -> bool {
    email.parse::<EmailAddress>().is_ok()
}

// 检查url是否合法
pub fn is_url(url: &str) -> bool {
    Url::parse(url).is_ok()
}

pub fn is_ip(ip: &str) -> bool {
    use std::net::IpAddr;
    match ip.parse::<IpAddr>() {
        Ok(IpAddr::V4(_)) => true,
        Ok(IpAddr::V6(_)) => true,
        Err(_) => false,
    }
}

pub fn is_ip_v4(ip: &str) -> bool {
    use std::net::IpAddr;
    match ip.parse::<IpAddr>() {
        Ok(IpAddr::V4(_)) => true,
        Ok(IpAddr::V6(_)) => false,
        Err(_) => false,
    }
}

pub fn is_ip_v6(ip: &str) -> bool {
    use std::net::IpAddr;
    match ip.parse::<IpAddr>() {
        Ok(IpAddr::V4(_)) => false,
        Ok(IpAddr::V6(_)) => true,
        Err(_) => false,
    }
}
