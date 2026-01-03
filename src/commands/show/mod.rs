use anyhow::Context;
use anyhow::Result;
use std::path::PathBuf;

use crate::core::utils::cert_utils::CertInfo;
use crate::core::{config::app_config::Config, utils::cert_utils::get_cert_info_path};

// 显示证书信息
pub fn show_cert_info(dry_run: bool, path: PathBuf) {
    // 检查路径是否存在
    if !path.exists() {
        println!("The path does not exist.");
        return;
    }

    // 路径不能为目录
    if path.is_dir() {
        println!("The path must not be a directory.");
        return;
    }

    // 获取证书信息
    let cert_info = get_cert_info(dry_run, path);

    // 打印信息并处理错误
    match cert_info {
        Ok(cert_info) => {
            println!("{}", cert_info);
        }
        Err(e) => {
            println!("{}", e);
        }
    }
}

// 显示证书的信息
fn get_cert_info(dry_run: bool, path: PathBuf) -> Result<CertInfo> {
    // 传入的文件路径是toml结尾, 把它当成配置文件解析, 获取期中的证书路径
    let cert_path = if path
        .extension()
        .map_or(false, |ext| ext.eq_ignore_ascii_case("toml"))
    {
        let config = Config::new(dry_run, &path).context("Failed to parse config file")?;
        config.target.cert_file
    } else {
        path
    };

    // 获取证书信息
    get_cert_info_path(&cert_path)
        .with_context(|| format!("Failed to read certificate: {:?}", cert_path))
}
