use std::{env, fs};

use crate::cli::param_enums::InitType;

// 初始化
pub fn init_config_file(init_type: &InitType) {
    // 初始化文件内容处理
    let config_content = match init_type {
        InitType::Http01 => http01_config_file(),
        InitType::Dns01 => dns01_config_file(),
        InitType::Full => full_config_file(),
    };

    // 获取当前的程序路径, 写入数据
    let path_buf = env::current_dir();
    if let Err(e) = path_buf {
        println!("Failed to get current directory, {}", e);
        return;
    }

    let mut path_buf = path_buf.unwrap();
    path_buf.push("config.toml");

    // 判断文件是否存在
    if path_buf.exists() {
        println!("Failed to write content to config.toml: the file already exists");
        return;
    }

    // 写内容到文件
    let result = fs::write(path_buf, config_content);
    if let Err(e) = result {
        println!("Failed to write to config.toml, {}", e);
        return;
    }
    println!("Initialization succeeded")
}

// 获取http01挑战配置文件内容
fn http01_config_file() -> String {
    r#"[target]
domains = [""]
#cert_file = "/etc/nginx/certs/fullchain.pem"
#key_file = "/etc/nginx/certs/privkey.pem"
#post_hook = "systemctl reload nginx"

[ca]
type = "letsencrypt"
account = "your@163.com"

[challenge]
type = "http01"

[challenge.http01]
webroot = "/var/www/acme-challenge/"

#[notice]
#enabled = true

#[notice.webhook]
#provider = "dingtalk"
#url = ""
#extra_param1="ROBOT""#
        .to_string()
}

// 获取http01挑战配置文件内容
fn dns01_config_file() -> String {
    r#"[target]
domains = [""]
#cert_file = "/etc/nginx/certs/fullchain.pem"
#key_file = "/etc/nginx/certs/privkey.pem"
#post_hook = "systemctl reload nginx"

[ca]
type = "letsencrypt"
account = "your@163.com"

[challenge]
type = "dns01"

[challenge.dns01]
provider = "dnsPod"
access_key_id = ""
access_key_secret = ""

#[notice]
#enabled = true

#[notice.webhook]
#provider = "weico"
#url = """#
        .to_string()
}

fn full_config_file() -> String {
    r#"[target]
domains = [""]
format = "pem"
#cert_file = "/etc/nginx/certs/fullchain.pem"
#key_file = "/etc/nginx/certs/privkey.pem"
renew_before_days = 30
auto_backup = true
#pre_hook = "echo hello"
#post_hook = "systemctl reload nginx"
log_level = "info"

[ca]
type = "letsencrypt"
account = "your@163.com"
account_token = "account_token.json"

[challenge]
type = "http01"

[challenge.http01]
webroot = "/var/www/acme-challenge/"

[challenge.dns01]
provider = "ali"
access_key_id = ""
access_key_secret = ""
#extra_param1 = "huawei_product_id"
#check_resolver = "1.1.1.1"

[notice]
enabled = true
level = "low"

[notice.webhook]
provider = "slack"
url = ""

[notice.email]
smtp = "smtp.163.com"
smtp_port = 465
username = "your@163.com"
password = ""
from = "your@163.com"
to = ["your@qq.com"]
#subject = "Certificate Auto-Renewal Notification""#
        .to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test_log::test]
    fn test_init() {
        init_config_file(&InitType::Full);
    }
}
