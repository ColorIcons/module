use std::{fs, path::Path};

use serde::Serialize;

use crate::{config::model::Config, core::types::Index};

#[derive(Debug, Serialize)]
struct CheckResult {
    updated: bool,
    old_generated_at: Option<u64>,
    new_generated_at: u64,
}

pub async fn check(
    config: &Config,
    local_index_path: &Path,
    json_mode: bool,
) -> anyhow::Result<()> {
    let base_url = &config.repo.base_url;
    let new_index: Index = reqwest::get(format!("{}/index.json", base_url.trim_end_matches('/')))
        .await?
        .json()
        .await?;

    let old_index = if local_index_path.exists() {
        Some(serde_json::from_str::<Index>(&fs::read_to_string(
            local_index_path,
        )?)?)
    } else {
        None
    };

    let icons_match = old_index
        .as_ref()
        .is_some_and(|old| old.icons == config.icons);

    let updated = match &old_index {
        Some(old) => old.generated_at < new_index.generated_at || !icons_match,
        None => true,
    };

    if json_mode {
        let result = CheckResult {
            updated,
            old_generated_at: old_index.as_ref().map(|i| i.generated_at),
            new_generated_at: new_index.generated_at,
        };
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        match &old_index {
            Some(old) => {
                if updated {
                    println!(
                        "Index updated: {} -> {}",
                        old.generated_at, new_index.generated_at
                    );
                } else {
                    println!("Index is up to date: {}", old.generated_at);
                }
            }
            None => println!(
                "No local index, new generated_at: {}",
                new_index.generated_at
            ),
        }
    }

    Ok(())
}
