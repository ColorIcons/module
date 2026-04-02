use std::path::PathBuf;

use clap::Args;

#[derive(Args)]
pub struct ListCmd {
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    #[arg(short, long)]
    pub json: bool,
}
