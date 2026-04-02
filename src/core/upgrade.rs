use crate::config::model::{Config, Icons};
use crate::core::types::{Index, Manifest};
use crate::utils::package::get_installed_packages;
use futures::stream::{FuturesUnordered, StreamExt};
use futures::FutureExt;
use reqwest::Client;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::{fs, io};
use tokio::fs as tokio_fs;
use tokio::io::AsyncWriteExt;

/// 输出统一格式
fn emit(json_mode: bool, value: serde_json::Value) {
    if json_mode {
        println!("{}", value);
    } else if let Some(msg) = value.get("message").and_then(|v| v.as_str()) {
        println!("{}", msg);
    }
    io::stdout().flush().ok();
}

/// 判断某个 variant 是否需要下载
fn should_download_variant(variant: &str, icons: &Icons) -> bool {
    match variant {
        "light" => icons.light,
        "dark" => icons.dark,
        "mat" => icons.mat,
        "monochrome" => icons.monochrome,
        _ => false,
    }
}

pub async fn upgrade(
    base_url: &str,
    storage_root: &Path,
    temp_path: &Path,
    index_path: &Path,
    config: &Config,
    json_mode: bool,
) -> anyhow::Result<()> {
    let installed = get_installed_packages();
    let client = Client::new();

    emit(
        json_mode,
        json!({
            "type":"stage",
            "value":"fetch",
            "message":"Fetching index.json"
        }),
    );

    // 获取 index.json
    let index_bytes = client
        .get(format!("{}/index.json", base_url.trim_end_matches('/')))
        .send()
        .await?
        .bytes()
        .await?;
    let index: Index = serde_json::from_slice(&index_bytes)?;

    let old_index: Option<Index> = if index_path.exists() {
        fs::read(index_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
    } else {
        None
    };

    emit(
        json_mode,
        json!({
            "type":"info",
            "value":"version",
            "message": format!("Index version: {}", index.repo_version)
        }),
    );

    // 预计算任务总数
    let mut total_pkg_files = 0usize;

    // GLOBAL
    let global_files_to_download: Vec<_> = index
        .global
        .files
        .iter()
        .filter(|(f, info)| {
            old_index.as_ref().is_none_or(|old| {
                old.global.files.get(*f).is_none_or(|old_info| {
                    old_info.sha256 != info.sha256 || old_info.size != info.size
                })
            })
        })
        .collect();

    let total_global_files = global_files_to_download.len();

    // PACKAGE
    for (pkg_name, new_pkg_info) in &index.packages {
        if !installed.contains_key(pkg_name) {
            continue;
        }

        let old_ver = old_index
            .as_ref()
            .and_then(|o| o.packages.get(pkg_name))
            .map(|p| p.version.clone());

        if let Some(old_ver) = old_ver && old_ver == new_pkg_info.version {
            continue;
        }

        let manifest_bytes = client
            .get(format!(
                "{}/{}",
                base_url.trim_end_matches('/'),
                new_pkg_info.manifest
            ))
            .send()
            .await?
            .bytes()
            .await?;

        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;

        total_pkg_files += manifest
            .files
            .iter()
            .filter(|mf| {
                mf.variant
                    .as_ref()
                    .is_none_or(|v| should_download_variant(v, &config.icons) || mf.required)
            })
            .count();
    }

    let total_tasks = total_global_files + total_pkg_files;
    let mut completed_tasks = 0usize;

    // 下载任务
    let mut tasks: FuturesUnordered<futures::future::BoxFuture<anyhow::Result<()>>> =
        FuturesUnordered::new();

    // GLOBAL
    for (file_name, new_file_info) in global_files_to_download {
        let temp_file = temp_path.join(file_name);
        let final_file = storage_root.join(file_name);
        let url = format!("{}/global/{}", base_url.trim_end_matches('/'), file_name);
        let sha = new_file_info.sha256.clone();
        let size = new_file_info.size;
        let client_clone = client.clone();
        let json_mode_clone = json_mode;
        let file_name_clone = file_name.clone();

        tasks.push(
            async move {
                emit(
                    json_mode_clone,
                    json!({
                        "type":"stage",
                        "value":"download_global",
                        "file": file_name_clone
                    }),
                );

                download_if_needed(&client_clone, &url, &temp_file, &sha, size, json_mode_clone)
                    .await?;

                fs::copy(&temp_file, &final_file)?;
                fs::set_permissions(&final_file, fs::Permissions::from_mode(0o644))?;

                Ok::<(), anyhow::Error>(())
            }
            .boxed(),
        );
    }

    // PACKAGE
    for (pkg_name, new_pkg_info) in &index.packages {
        if !installed.contains_key(pkg_name) {
            continue;
        }

        let old_ver = old_index
            .as_ref()
            .and_then(|o| o.packages.get(pkg_name))
            .map(|p| p.version.clone());

        if let Some(old_ver) = old_ver && old_ver == new_pkg_info.version {
            continue;
        }

        let manifest_bytes = client
            .get(format!(
                "{}/{}",
                base_url.trim_end_matches('/'),
                new_pkg_info.manifest
            ))
            .send()
            .await?
            .bytes()
            .await?;

        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;

        let pkg_temp_dir = temp_path.join(pkg_name);
        let pkg_final_dir = storage_root.join(pkg_name);
        fs::create_dir_all(&pkg_temp_dir)?;
        fs::create_dir_all(&pkg_final_dir)?;

        for mf in &manifest.files {
            if let Some(var) = &mf.variant && !should_download_variant(var, &config.icons) && !mf.required {
                continue;
            }

            let file_url = format!(
                "{}/packages/{}/{}",
                base_url.trim_end_matches('/'),
                pkg_name,
                mf.file
            );

            let temp_file = pkg_temp_dir.join(&mf.file);
            let final_file = pkg_final_dir.join(&mf.file);
            let sha = mf.sha256.clone().unwrap_or_default();
            let size = mf.size.unwrap_or(0);
            let client_clone = client.clone();
            let json_mode_clone = json_mode;
            let pkg_name_clone = pkg_name.clone();
            let file_name = mf.file.clone();

            tasks.push(
                async move {
                    emit(
                        json_mode_clone,
                        json!({
                            "type":"stage",
                            "value":"download_package",
                            "package": pkg_name_clone,
                            "file": file_name
                        }),
                    );

                    download_if_needed(
                        &client_clone,
                        &file_url,
                        &temp_file,
                        &sha,
                        size,
                        json_mode_clone,
                    )
                    .await?;

                    if let Some(parent) = final_file.parent() {
                        fs::create_dir_all(parent)?;
                        fs::set_permissions(parent, fs::Permissions::from_mode(0o755))?;
                    }

                    fs::copy(&temp_file, &final_file)?;
                    fs::set_permissions(&final_file, fs::Permissions::from_mode(0o644))?;

                    Ok::<(), anyhow::Error>(())
                }
                .boxed(),
            );
        }
    }

    while let Some(res) = tasks.next().await {
        res?;

        completed_tasks += 1;

        let progress = if total_tasks == 0 {
            1.0
        } else {
            completed_tasks as f64 / total_tasks as f64
        };

        emit(
            json_mode,
            json!({
                "type":"progress",
                "value": progress
            }),
        );
    }

    tokio_fs::write(index_path, serde_json::to_vec(&index)?).await?;

    emit(
        json_mode,
        json!({
            "type":"info",
            "message":"index.json updated"
        }),
    );

    if temp_path.exists() {
        tokio_fs::remove_dir_all(temp_path).await.ok();
        emit(
            json_mode,
            json!({
                "type":"info",
                "message":"Cleaned temporary files"
            }),
        );
    }

    emit(
        json_mode,
        json!({
            "type":"done",
            "message":"Upgrade complete"
        }),
    );

    Ok(())
}

/// 下载
async fn download_if_needed(
    client: &Client,
    url: &str,
    path: &Path,
    sha256: &str,
    size: u64,
    json_mode: bool,
) -> anyhow::Result<()> {
    if path.exists() {
        let bytes = tokio_fs::read(&path).await?;
        let hash = Sha256::digest(&bytes);
        if hex::encode(hash) == sha256 {
            return Ok(());
        }
    }

    emit(
        json_mode,
        json!({
            "type":"stage",
            "value":"download",
            "url": url
        }),
    );

    let resp = client.get(url).send().await?;
    let bytes = resp.bytes().await?;

    if size > 0 && bytes.len() as u64 != size {
        emit(
            json_mode,
            json!({
                "type":"info",
                "message": format!(
                    "[WARN] size mismatch: {} != {} ({})",
                    bytes.len(),
                    size,
                    url
                )
            }),
        );
    }

    let mut file = tokio_fs::File::create(path).await?;
    file.write_all(&bytes).await?;

    emit(
        json_mode,
        json!({
            "type":"info",
            "message": format!("Downloaded: {}", path.display())
        }),
    );

    Ok(())
}
