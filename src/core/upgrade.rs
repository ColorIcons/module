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

/// 核心升级函数
pub async fn upgrade(
    base_url: &str,
    storage_root: &Path,
    temp_path: &Path,
    index_path: &Path,
    config: &Config,
    json_mode: bool,
) -> anyhow::Result<()> {
    emit(
        json_mode,
        json!({"type":"stage","value":"init","message":"Preparing directories"}),
    );

    let installed = get_installed_packages();

    emit(
        json_mode,
        json!({"type":"stage","value":"fetch_index","message":"Fetching index.json"}),
    );

    let client = Client::new();
    let index_bytes = client
        .get(format!("{}/index.json", base_url.trim_end_matches('/')))
        .send()
        .await?
        .bytes()
        .await?;
    let index: Index = serde_json::from_slice(&index_bytes)?;

    // 读取旧的 index.json
    let old_index: Option<Index> = if index_path.exists() {
        match fs::read(index_path) {
            Ok(old_bytes) => serde_json::from_slice(&old_bytes).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    emit(
        json_mode,
        json!({"type":"info","value":"version","message":format!("Index version: {}", index.repo_version)}),
    );

    let mut tasks: FuturesUnordered<futures::future::BoxFuture<anyhow::Result<()>>> =
        FuturesUnordered::new();

    // 下载 global 文件
    for (file_name, new_file_info) in &index.global.files {
        let should_download = match &old_index {
            Some(old) => {
                if let Some(old_info) = old.global.files.get(file_name) {
                    old_info.sha256 != new_file_info.sha256 || old_info.size != new_file_info.size
                } else {
                    true
                }
            }
            None => true,
        };

        if !should_download {
            emit(
                json_mode,
                json!({"type":"info","message":format!("Skipped (up-to-date): {}", file_name)}),
            );
            continue;
        }

        let temp_file = temp_path.join(file_name);
        let final_file = storage_root.join(file_name);
        let url = format!("{}/global/{}", base_url.trim_end_matches('/'), file_name);
        let sha = new_file_info.sha256.clone();
        let size = new_file_info.size;
        let client_clone = client.clone();
        let json_mode_clone = json_mode;

        tasks.push(
            async move {
                emit(
                    json_mode_clone,
                    json!({"type":"stage","value":"download_global","file":file_name}),
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

    // 下载已安装应用的图标
    for (pkg_name, new_pkg_info) in &index.packages {
        if !installed.contains_key(pkg_name) {
            continue;
        }

        let old_pkg_version = old_index
            .as_ref()
            .and_then(|o| o.packages.get(pkg_name))
            .map(|p| p.version.clone());

        if let Some(old_ver) = old_pkg_version
            && old_ver == new_pkg_info.version {
                emit(json_mode, json!({
                    "type":"info",
                    "package": pkg_name,
                    "message": "Skipped (version up-to-date)"
                }));
                continue;
            }

        // 下载新 manifest
        let manifest_url = format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            new_pkg_info.manifest
        );
        let manifest_bytes = client.get(&manifest_url).send().await?.bytes().await?;
        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;

        let pkg_temp_dir = temp_path.join(pkg_name);
        let pkg_final_dir = storage_root.join(pkg_name);
        fs::create_dir_all(&pkg_temp_dir)?;
        fs::create_dir_all(&pkg_final_dir)?;
        fs::set_permissions(&pkg_final_dir, fs::Permissions::from_mode(0o755))?;

        for mf in &manifest.files {
            if let Some(var) = &mf.variant
                && !should_download_variant(var, &config.icons) && !mf.required
            {
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
            let file_name = mf.file.clone();
            let pkg_name_clone = pkg_name.clone();

            tasks.push(
                async move {
                    emit(
                        json_mode_clone,
                        json!({
                            "type":"stage",
                            "value":"download_package",
                            "package":pkg_name_clone,
                            "file":file_name
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

    // 等待所有下载任务完成
    while let Some(res) = tasks.next().await {
        res?;
    }

    // 更新本地 index.json
    tokio_fs::write(index_path, serde_json::to_vec(&index)?).await?;
    emit(
        json_mode,
        json!({"type":"info","message":"index.json updated"}),
    );

    // 清理临时目录
    if temp_path.exists() {
        tokio_fs::remove_dir_all(temp_path).await.ok();
        emit(
            json_mode,
            json!({"type":"info","message":"Cleaned temporary files"}),
        );
    }

    emit(
        json_mode,
        json!({"type":"done","message":"Upgrade complete"}),
    );

    Ok(())
}

/// 下载并校验 SHA256
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
        let hash_hex = hex::encode(hash);
        if hash_hex == sha256 {
            emit(
                json_mode,
                json!({"type":"info","message":format!("Skipped (up-to-date): {}", path.display())}),
            );
            return Ok(());
        }
    }

    emit(
        json_mode,
        json!({"type":"stage","value":"download","url":url}),
    );

    let resp = client.get(url).send().await?;
    let bytes = resp.bytes().await?;
    if bytes.len() as u64 != size {
        eprintln!(
            "[警告] {} 大小不匹配: {} != {}",
            path.display(),
            bytes.len(),
            size
        );
    }

    let mut file = tokio_fs::File::create(path).await?;
    file.write_all(&bytes).await?;
    emit(
        json_mode,
        json!({"type":"info","message":format!("Downloaded: {}", path.display())}),
    );

    Ok(())
}
