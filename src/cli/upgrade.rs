use clap::Args;

#[derive(Args)]
pub struct UpgradeCmd {
    #[arg(short, long)]
    pub json: bool,
}
