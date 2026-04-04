use crate::config::model::{Config, Icons};
use crate::core::types::{Index, Manifest};
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
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

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

    let mut resp = client.get(url).send().await?.error_for_status()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
        fs::set_permissions(parent, Permissions::from_mode(0o755))
            .await
            .ok();
    }

    let mut file = fs::File::create(path).await?;
    let mut hasher = Sha256::new();

    while let Some(chunk) = resp.chunk().await? {
        if !expected_sha.is_empty() {
            hasher.update(&chunk);
        }
        file.write_all(&chunk).await?;
    }
    file.flush().await?;

    if !expected_sha.is_empty() {
        let hash = hex::encode(hasher.finalize());
        if hash != expected_sha {
            anyhow::bail!(
                "SHA256 mismatch for {}: expected {}, got {}",
                path.display(),
                expected_sha,
                hash
            );
        }
    }

    fs::set_permissions(path, Permissions::from_mode(0o644)).await?;
    Ok(())
}

fn should_download_package(force_update: bool, old_ver: Option<&String>, new_ver: &str) -> bool {
    force_update || old_ver.is_none_or(|v| v != new_ver)
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
    let base_url = base_url.trim_end_matches('/');

    let concurrency = config.network.concurrency.max(1);
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let local_index: Option<Index> = if index_path.exists() {
        fs::read(index_path)
            .await
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
    } else {
        None
    };

    let force_update = local_index
        .as_ref()
        .is_none_or(|idx| idx.icons != config.icons);

    emit(
        json_mode,
        json!({"type":"stage","value":"fetch","message":"Fetching index.json"}),
    );
    let index_bytes = client
        .get(format!("{}/index.json", base_url))
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let remote_index: Index = serde_json::from_slice(&index_bytes)?;

    if !force_update && let Some(local) = &local_index
        && local.repo_version >= remote_index.repo_version && local.generated_at >= remote_index.generated_at {
            emit(json_mode, json!({"type":"done","message":"Already up-to-date"}));
            return Ok(());
        }

    let mut download_queue = Vec::new();

    // Global Files
    for (name, info) in &remote_index.global.files {
        let old = local_index
            .as_ref()
            .and_then(|idx| idx.global.files.get(name));
        if old.is_none_or(|o| o.sha256 != info.sha256) {
            download_queue.push((
                format!("{}/global/{}", base_url, name),
                storage_root.join(name),
                info.sha256.clone(),
            ));
        }
    }

    // Global Packages
    for (pkg_name, pkg) in &remote_index.global.packages {
        let old_pkg = local_index
            .as_ref()
            .and_then(|idx| idx.global.packages.get(pkg_name));
        for (file_name, info) in &pkg.files {
            let old_file = old_pkg.and_then(|p| p.files.get(file_name));
            if old_file.is_none_or(|o| o.sha256 != info.sha256) {
                download_queue.push((
                    format!("{}/global/{}/{}", base_url, pkg_name, file_name),
                    storage_root.join(pkg_name).join(file_name),
                    info.sha256.clone(),
                ));
            }
        }
    }

    let mut package_download_count = 0;
    for (pkg_name, new_info) in &remote_index.packages {
        if !installed.contains_key(pkg_name) {
            continue;
        }
        let old_ver = local_index
            .as_ref()
            .and_then(|idx| idx.packages.get(pkg_name))
            .map(|p| &p.version);

        if should_download_package(force_update, old_ver, &new_info.version) {
            let manifest_url = format!("{}/{}", base_url, new_info.manifest);
            let m_bytes = client
                .get(manifest_url)
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await?;
            let manifest: Manifest = serde_json::from_slice(&m_bytes)?;

            for mf in manifest.files {
                if let Some(v) = &mf.variant
                    && !should_download_variant(v, &config.icons) && !mf.required {
                        continue;
                    }
                download_queue.push((
                    format!("{}/packages/{}/{}", base_url, pkg_name, mf.file),
                    storage_root.join(pkg_name).join(&mf.file),
                    mf.sha256.clone().unwrap_or_default(),
                ));
            }
            package_download_count += 1;
        }
    }

    let total_tasks = download_queue.len();
    let completed_tasks = Arc::new(AtomicUsize::new(0));
    let mut tasks = FuturesUnordered::new();

    for (url, path, sha) in download_queue {
        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);
        let completed = Arc::clone(&completed_tasks);

        tasks.push(
            async move {
                let _permit = semaphore.acquire_owned().await.unwrap(); // 只有在下载瞬间才占用信号量
                download_file(&client, &url, &path, &sha, json_mode).await?;

                let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
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

    let mut final_index = remote_index;
    final_index.icons = config.icons.clone();
    fs::write(index_path, serde_json::to_vec(&final_index)?).await?;

    emit(
        json_mode,
        json!({
            "type":"done",
            "message":"Upgrade complete",
            "packages_downloaded": package_download_count
        }),
    );

    Ok(())
}
