use crate::{
    cli::upgrade::UpgradeCmd,
    config::{loader, model},
    core::upgrade,
};

pub async fn run(cmd: UpgradeCmd) -> anyhow::Result<()> {
    let path = model::CONFIG_PATH.clone();

    if !path.exists() {
        anyhow::bail!("config not exists");
    }

    let config = loader::load(path)?;
    let base_url = &config.repo.base_url;
    let storage_root = model::STORAGE_ROOT.clone();
    let index_path = model::INDEX_FILE_PATH.clone();
    let package_list_path = model::PACKAGE_LIST_PATH.clone();

    upgrade::upgrade(
        base_url,
        &storage_root,
        &index_path,
        &package_list_path,
        &config,
        cmd.json,
    )
    .await?;

    Ok(())
}
