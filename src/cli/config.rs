use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    Init(()),
    Get(GetCmd),
    Set(SetCmd),
}

#[derive(Args, Debug)]
pub struct GetCmd {
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SetCmd {
    #[arg(long)]
    pub base_url: Option<String>,

    #[arg(long)]
    pub concurrency: Option<usize>,

    #[arg(long)]
    pub light: Option<bool>,

    #[arg(long)]
    pub dark: Option<bool>,

    #[arg(long)]
    pub mat: Option<bool>,

    #[arg(long)]
    pub monochrome: Option<bool>,
}
