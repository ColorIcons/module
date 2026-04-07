use crate::core::list;
use tokio::fs;

use crate::{cli::package::PackageListCmd, config::model, core::types::Index};

pub async fn run_package_list(cmd: PackageListCmd) -> anyhow::Result<()> {
    let json_mode = cmd.json;
    let index_path = model::INDEX_FILE_PATH.clone();

    let index: Index = if index_path.exists() {
        let index_bytes = fs::read(index_path).await?;
        serde_json::from_slice(&index_bytes)?
    } else {
        Index::default()
    };

    let storage_path = model::STORAGE_ROOT.clone();
    let list = list::get_packages_list(&index, &storage_path).await?;

    if json_mode {
        println!("{}", serde_json::to_string_pretty(&list)?);
    } else {
        println!("{:<40} {:<10}", "PACKAGE", "ADAPTED");
        println!("{}", "-".repeat(50));
        for item in list {
            println!(
                "{:<40} {:<10}",
                item.package_name,
                if item.is_adapted { "YES" } else { "-" }
            );
        }
    }
    Ok(())
}
