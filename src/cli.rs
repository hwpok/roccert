use clap::{Parser, Subcommand};
use param_enums::{CommandType, Lang};
use std::path::PathBuf;

use crate::cli::param_enums::InitType;

/// A simple yet efficient certificate renewal tool written in Rust
#[derive(Parser, Debug)]
#[command(name = "roccert", version, about, long_about = None)]
pub struct Commands {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: SubCommands,
}

#[derive(Debug, Subcommand)]
pub enum SubCommands {
    /// Generate document for roccert
    #[command(about = "Generate document for roccert")]
    Docs {
        /// Documentation language (zh or en)
        #[arg(short = 'l', long = "lang", default_value = "zh")]
        lang: Lang,
    },

    /// Initialize a configuration file
    #[command(about = "Generate config.toml based on init type")]
    Init {
        /// init type (http01 or dns01 or full)
        #[arg(short = 'i', long = "init-type", default_value = "http01")]
        init_type: InitType,
    },

    /// Test the "new" and "renew" commands
    #[command(about = "Starting check configuration and dry run")]
    Test {
        /// The type of command to execute: either "new" or "renew"
        #[arg(short = 'c', long = "command-type", default_value = "new")]
        command_type: CommandType,
        /// Path to config file (default: ./config.toml)
        #[arg(value_name = "CONFIG_FILE", default_value = "config.toml")]
        config_file: PathBuf,
    },

    /// Obtain a new certificate
    #[command(about = "Obtain a new certificate")]
    New {
        /// Path to config file (default: ./config.toml)
        #[arg(value_name = "CONFIG_FILE", default_value = "config.toml")]
        config_file: PathBuf,
    },

    /// Renew existing certificates
    #[command(about = "Renew existing certificates")]
    Renew {
        /// Path to config file (default: ./config.toml)
        #[arg(value_name = "CONFIG_FILE", default_value = "config.toml")]
        config_file: PathBuf,
    },

    /// Display certificate details
    Show {
        /// Test certificate
        #[arg(short = 't', long = "test", action = clap::ArgAction::SetTrue)]
        test: bool,
        /// Path to config.toml or certificate file (.pem)
        #[arg(value_name = "CERT_FILE", default_value = "config.toml")]
        file: PathBuf,
    },
}

pub mod param_enums {
    use clap::ValueEnum;

    #[derive(ValueEnum, Clone, Debug)]
    pub enum Lang {
        Zh,
        En,
    }

    #[derive(ValueEnum, Clone, Debug)]
    pub enum ChallengeType {
        Http01,
        Dns01,
    }

    #[derive(ValueEnum, Clone, Debug)]
    pub enum CommandType {
        New,
        Renew,
    }

    #[derive(ValueEnum, Clone, Debug)]
    pub enum InitType {
        Http01,
        Dns01,
        Full,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_new_command_defaults() {
        let cli = Commands::parse_from(["rolcert", "new"]);
        match cli.command {
            SubCommands::New { config_file } => {
                assert_eq!(config_file, PathBuf::from("config.toml"));
            }
            _ => panic!("Expected New command"),
        }
        assert_eq!(cli.verbose, false);
    }

    #[test]
    fn test_new_command_custom_config() {
        let cli = Commands::parse_from(["rolcert", "new", "my-config.toml"]);
        match cli.command {
            SubCommands::New { config_file, .. } => {
                assert_eq!(config_file, PathBuf::from("my-config.toml"));
            }
            _ => panic!("Expected New command"),
        }
    }
}
