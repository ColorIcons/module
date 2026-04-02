use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct ListCmd {
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    #[arg(short, long)]
    pub json: bool,
}
