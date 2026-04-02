use clap::Args;

#[derive(Args)]
pub struct ListCmd {
    #[arg(short, long)]
    pub json: bool,
}
