pub mod docs;
pub mod init;
pub mod renew;
pub mod show;

use crate::{
    cli::{SubCommands, param_enums::CommandType},
    commands::renew::{new_cert, renew_cert},
};

/// 分发并执行子命令
pub async fn dispatch(command: SubCommands) {
    match command {
        SubCommands::Docs { lang } => {
            log::debug!("Using docs, lang: {:?}", lang);
            docs::gen_docs(&lang);
        }
        SubCommands::Init { init_type } => {
            log::debug!("Using init, init type: {:?}", init_type);
            init::init_config_file(&init_type);
        }
        SubCommands::Test {
            command_type,
            config_file,
        } => {
            log::debug!("Using Test, command type: {}", config_file.display());
            match command_type {
                CommandType::New => new_cert(&config_file, true).await,
                CommandType::Renew => renew_cert(&config_file, true).await,
            }
        }
        SubCommands::New { config_file } => {
            log::debug!("Using new: {}", config_file.display());
            new_cert(&config_file, false).await;
        }
        SubCommands::Renew { config_file } => {
            log::debug!("Using renew: {}", config_file.display());
            renew_cert(&config_file, false).await;
        }
        SubCommands::Show { test, file } => {
            log::debug!("show certificate info, {}", file.display());
            show::show_cert_info(test, file);
        }
    };
}
