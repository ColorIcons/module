use crate::config::model::{Config, Icons};
use crate::core::types::{FileInfo, Index, Manifest};
use crate::utils::package::get_installed_packages;
use futures::stream::{FuturesUnordered, StreamExt};
use futures::FutureExt;
use reqwest::Client;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// 输出统一格式
fn emit(json_mode: bool, value: serde_json::Value) {
    if json_mode {
        println!("{}", value);
    } else if let Some(msg) = value.get("message").and_then(|v| v.as_str()) {
        println!("{}", msg);
    }
}

/// 判断 variant 是否需要下载
fn should_download_variant(variant: &str, icons: &Icons) -> bool {
    match variant {
        "light" => icons.light,
        "dark" => icons.dark,
        "mat" => icons.mat,
        "monochrome" => icons.monochrome,
        _ => false,
    }
}

async fn download_file(
    client: &Client,
    url: &str,
    path: &Path,
    expected_sha: &str,
    json_mode: bool,
) -> anyhow::Result<()> {
    emit(
        json_mode,
        json!({"type":"stage","value":"download","message": url}),
    );

    let resp = client.get(url).send().await?.error_for_status()?;
    let bytes = resp.bytes().await?;

    // 校验 SHA256
    if !expected_sha.is_empty() {
        let hash = Sha256::digest(&bytes);
        if hex::encode(hash) != expected_sha {
            anyhow::bail!("SHA256 mismatch for {}", path.display());
        }
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
        fs::set_permissions(parent, Permissions::from_mode(0o755))
            .await
            .ok();
    }

    // 写文件
    let mut file = fs::File::create(path).await?;
    file.write_all(&bytes).await?;
    fs::set_permissions(path, Permissions::from_mode(0o644)).await?;

    Ok(())
}

pub async fn upgrade(
    base_url: &str,
    storage_root: &Path,
    index_path: &Path,
    config: &Config,
    json_mode: bool,
) -> anyhow::Result<()> {
    let installed = get_installed_packages();
    let client = Client::new();

    emit(
        json_mode,
        json!({"type":"stage","value":"fetch","message":"Fetching index.json"}),
    );

    let start_fetch = Instant::now();
    let index_bytes = client
        .get(format!("{}/index.json", base_url.trim_end_matches('/')))
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let index: Index = serde_json::from_slice(&index_bytes)?;
    emit(
        json_mode,
        json!({"type":"time","stage":"fetch_index","duration_ms": start_fetch.elapsed().as_millis()}),
    );

    // 读取旧 index
    let old_index: Option<Index> = if index_path.exists() {
        fs::read(index_path)
            .await
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
    } else {
        None
    };

    let mut total_tasks = 0usize;

    let should_download = |old: Option<&FileInfo>, new: &FileInfo| match old {
        Some(o) => o.sha256 != new.sha256 || o.size != new.size,
        None => true,
    };

    // 统计任务
    for (file_name, info) in &index.global.files {
        if should_download(
            old_index
                .as_ref()
                .and_then(|o| o.global.files.get(file_name.as_str())),
            info,
        ) {
            total_tasks += 1;
        }
    }

    for (pkg_name, global_pkg) in &index.global.packages {
        let old_pkg = old_index
            .as_ref()
            .and_then(|o| o.global.packages.get(pkg_name.as_str()));
        for (file_name, info) in &global_pkg.files {
            if should_download(old_pkg.and_then(|p| p.files.get(file_name.as_str())), info) {
                total_tasks += 1;
            }
        }
    }

    for (pkg_name, new_pkg_info) in &index.packages {
        if !installed.contains_key(pkg_name) {
            continue;
        }
        let old_ver = old_index
            .as_ref()
            .and_then(|o| o.packages.get(pkg_name.as_str()))
            .map(|p| p.version.clone());
        if let Some(old_ver) = old_ver && old_ver == new_pkg_info.version {
            continue;
        }
        total_tasks += 1;
    }

    let completed_tasks = Arc::new(AtomicUsize::new(0));
    let downloaded_packages = Arc::new(AtomicUsize::new(0));
    let mut tasks: FuturesUnordered<_> = FuturesUnordered::new();

    // global files
    for (file_name, info) in &index.global.files {
        if !should_download(
            old_index
                .as_ref()
                .and_then(|o| o.global.files.get(file_name.as_str())),
            info,
        ) {
            continue;
        }

        let final_file = storage_root.join(file_name);
        let url = format!("{}/global/{}", base_url.trim_end_matches('/'), file_name);
        let sha = info.sha256.clone();
        let client_clone = client.clone();
        let completed_clone = Arc::clone(&completed_tasks);

        tasks.push(
            async move {
                download_file(&client_clone, &url, &final_file, &sha, json_mode).await?;
                let done = completed_clone.fetch_add(1, Ordering::SeqCst) + 1;
                emit(
                    json_mode,
                    json!({"type":"progress","value": done as f64 / total_tasks as f64}),
                );
                Ok::<(), anyhow::Error>(())
            }
            .boxed(),
        );
    }

    for (pkg_name, global_pkg) in &index.global.packages {
        let old_pkg = old_index
            .as_ref()
            .and_then(|o| o.global.packages.get(pkg_name.as_str()));

        for (file_name, info) in &global_pkg.files {
            if !should_download(old_pkg.and_then(|p| p.files.get(file_name.as_str())), info) {
                continue;
            }

            let final_file = storage_root.join(pkg_name).join(file_name);
            let url = format!(
                "{}/global/{}/{}",
                base_url.trim_end_matches('/'),
                pkg_name,
                file_name
            );
            let sha = info.sha256.clone();
            let client_clone = client.clone();
            let completed_clone = Arc::clone(&completed_tasks);

            tasks.push(
                async move {
                    download_file(&client_clone, &url, &final_file, &sha, json_mode).await?;
                    let done = completed_clone.fetch_add(1, Ordering::SeqCst) + 1;
                    emit(
                        json_mode,
                        json!({"type":"progress","value": done as f64 / total_tasks as f64}),
                    );
                    Ok::<(), anyhow::Error>(())
                }
                .boxed(),
            );
        }
    }

    for (pkg_name, new_pkg_info) in &index.packages {
        if !installed.contains_key(pkg_name) {
            continue;
        }

        let old_ver = old_index
            .as_ref()
            .and_then(|o| o.packages.get(pkg_name.as_str()))
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
            .error_for_status()?
            .bytes()
            .await?;

        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;

        let mut pkg_tasks: FuturesUnordered<_> = FuturesUnordered::new();

        for mf in &manifest.files {
            if let Some(var) = &mf.variant
                && !should_download_variant(var, &config.icons)
                && !mf.required
            {
                continue;
            }

            let final_file = storage_root.join(pkg_name).join(&mf.file);
            let url = format!(
                "{}/packages/{}/{}",
                base_url.trim_end_matches('/'),
                pkg_name,
                mf.file
            );
            let sha = mf.sha256.clone().unwrap_or_default();
            let client_clone = client.clone();

            pkg_tasks.push(async move {
                download_file(&client_clone, &url, &final_file, &sha, json_mode).await
            }.boxed());
        }

        let completed_clone = Arc::clone(&completed_tasks);
        let downloaded_clone = Arc::clone(&downloaded_packages);

        tasks.push(
            async move {
                while let Some(res) = pkg_tasks.next().await {
                    res?;
                }
                let done = completed_clone.fetch_add(1, Ordering::SeqCst) + 1;
                downloaded_clone.fetch_add(1, Ordering::SeqCst);
                emit(
                    json_mode,
                    json!({"type":"progress","value": done as f64 / total_tasks as f64}),
                );
                Ok::<(), anyhow::Error>(())
            }
            .boxed(),
        );
    }

    while let Some(res) = tasks.next().await {
        res?;
    }

    fs::write(index_path, serde_json::to_vec(&index)?).await?;

    let pkg_count = downloaded_packages.load(Ordering::SeqCst);

    emit(
        json_mode,
        json!({
            "type":"done",
            "message":"Upgrade complete",
            "packages_downloaded": pkg_count
        }),
    );

    Ok(())
}
