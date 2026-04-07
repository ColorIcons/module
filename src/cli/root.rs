use crate::cli::{
    check::CheckCmd, config::ConfigCmd, list::ListCmd, package::PackageCmd, upgrade::UpgradeCmd,
};
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
    /// Check for updates
    Check(CheckCmd),
    /// List installed icons
    List(ListCmd),
    /// Upgrade installed icons
    Upgrade(UpgradeCmd),
    /// Config
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
    /// Package
    Package {
        #[command(subcommand)]
        cmd: PackageCmd,
    },
}
