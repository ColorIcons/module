use crate::{
    cli::check::CheckCmd,
    config::{loader, model},
    core::check,
};

pub async fn run(cmd: CheckCmd) -> anyhow::Result<()> {
    let path = model::CONFIG_PATH.clone();

    if !path.exists() {
        anyhow::bail!("config not exists");
    }

    let config = loader::load(path)?;
    let index = model::INDEX_FILE_PATH.clone();

    check::check(&config, &index, cmd.json).await?;

    Ok(())
}
