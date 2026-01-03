use crate::{cli::Commands, commands::dispatch};
use anyhow::Result;
use clap::Parser;
use log::LevelFilter;
use rustls::crypto::{CryptoProvider, ring::default_provider};
use simple_logger::SimpleLogger;

pub mod cli;
pub mod commands;
pub mod core;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // 为 rustls 指定加密算法实现, unknow-linux-musl
    CryptoProvider::install_default(default_provider()).unwrap();

    // 获取显示日志参数
    let verbose = std::env::args().any(|arg| arg == "-v" || arg == "--verbose");

    // 设置日志级别
    SimpleLogger::new()
        .with_level(if verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        })
        .init()
        .unwrap();

    // 解析命令, 分发子命令
    let cli = Commands::parse();
    dispatch(cli.command).await;
    Ok(())
}
