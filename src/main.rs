mod app;
mod cli;
mod config;
mod core;
mod utils;

use crate::cli::{config::ConfigCmd, root::Commands};
use clap::Parser;
use cli::root::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check(cmd) => app::check::run(cmd).await?,
        Commands::List(cmd) => app::list::run(cmd)?,
        Commands::Config { cmd } => match cmd {
            ConfigCmd::Init(()) => app::config::init()?,
            ConfigCmd::Get(c) => app::config::get(c)?,
            ConfigCmd::Set(c) => app::config::set(c)?,
        },
    }
    Ok(())
}
