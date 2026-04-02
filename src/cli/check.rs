use std::path::PathBuf;

#[derive(clap::Args)]
pub struct CheckCmd {
    #[arg(short, long)]
    pub json: bool,
}
