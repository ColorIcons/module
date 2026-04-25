use serde::Serialize;
use std::{collections::HashSet, path::Path};
use tokio::fs; // 切换到异步 fs

use crate::{config::model::Config, core::types::Index, utils};

#[derive(Debug, Serialize)]
struct CheckResult {
    updated: bool,
    old_generated_at: Option<u64>,
    new_generated_at: u64,
    reasons: Vec<&'static str>,
}

async fn load_package_list(path: &Path) -> Option<HashSet<String>> {
    if !path.exists() {
        return None;
    }

    let data = fs::read(path).await.ok()?;
    serde_json::from_slice::<HashSet<String>>(&data).ok()
}

pub async fn check(
    config: &Config,
    local_index_path: &Path,
    packages_list_path: &Path,
    json_mode: bool,
) -> anyhow::Result<()> {
    let base_url = &config.repo.base_url;

    let new_index: Index = reqwest::get(format!("{}/index.json", base_url.trim_end_matches('/')))
        .await?
        .error_for_status()?
        .json()
        .await?;

    let old_index: Option<Index> = if local_index_path.exists() {
        let content = fs::read(local_index_path).await?;
        serde_json::from_slice(&content).ok()
    } else {
        None
    };

    let installed_map = utils::package::get_installed_packages();
    let installed_set: HashSet<String> = installed_map.keys().cloned().collect();

    let old_set = load_package_list(packages_list_path).await;

    let package_list_changed = match &old_set {
        Some(old) => *old != installed_set,
        None => true,
    };

    let icons_match = old_index
        .as_ref()
        .is_some_and(|old| old.icons == config.icons);

    let mut reasons = Vec::new();

    match &old_index {
        Some(old) => {
            if old.repo_version < new_index.repo_version {
                reasons.push("repo version changed");
            }
            if old.generated_at < new_index.generated_at {
                reasons.push("index updated");
            }
            if !icons_match {
                reasons.push("config changed");
            }
            if package_list_changed {
                reasons.push("package list changed");
            }
        }
        None => {
            reasons.push("no local index");
        }
    }

    let updated = !reasons.is_empty();

    if json_mode {
        let result = CheckResult {
            updated,
            old_generated_at: old_index.as_ref().map(|i| i.generated_at),
            new_generated_at: new_index.generated_at,
            reasons,
        };
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }
    match &old_index {
        Some(old) => {
            if updated {
                let reason_str = if reasons.is_empty() {
                    "".to_string()
                } else {
                    format!(" ({})", reasons.join(", "))
                };

                println!(
                    "Index update available: {} -> {}{}",
                    old.generated_at, new_index.generated_at, reason_str
                );
            } else {
                println!("Index is up to date: {}", old.generated_at);
            }
        }
        None => println!(
            "No local index found. Remote index generated at: {}",
            new_index.generated_at
        ),
    }

    Ok(())
}
