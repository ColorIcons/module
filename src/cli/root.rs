use clap::{Parser, Subcommand};

use crate::cli::{config::ConfigCmd, list::ListCmd};

#[derive(Parser)]
#[command(name = "cip")]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    List(ListCmd),
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
}
