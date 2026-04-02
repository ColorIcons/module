use crate::cli::{check::CheckCmd, config::ConfigCmd, list::ListCmd, upgrade::UpgradeCmd};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cip")]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Check(CheckCmd),
    List(ListCmd),
    Upgrade(UpgradeCmd),
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
}
