use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum PackageCmd {
    List(PackageListCmd),
}

#[derive(Args)]
pub struct PackageListCmd {
    #[arg(short, long)]
    pub json: bool,
}
