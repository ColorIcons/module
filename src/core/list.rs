use crate::{
    core::types::Index,
    utils::{monet_scan, package::get_installed_packages},
};
use rayon::prelude::*;
use serde::Serialize;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::Path,
    process::Command,
};
use tokio::fs;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    pub name: String,
    pub light_icon: Option<String>,
    pub dark_icon: Option<String>,
    pub monet_icon: Option<String>,
    pub mat_icon: Option<String>,
    pub is_adapted: bool,
    pub is_monet_supported_natively: bool,
}

#[derive(Debug, Serialize)]
pub struct PackageMatch {
    pub pkg_name: String,
    pub apk_path: String,
    pub is_global: bool,
}

#[derive(Debug, Serialize)]
pub struct PackageInfo {
    pub package_name: String,
    pub is_adapted: bool,
}

async fn get_installed_packages_with_global(index: &Index) -> anyhow::Result<Vec<PackageMatch>> {
    let output = Command::new("pm")
        .args(["list", "packages", "-f"])
        .output()
        .expect("无法执行 pm list packages");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut all_installed = HashMap::new();
    for line in stdout.lines() {
        if let Some((apk_part, pkg_name)) = line.rsplit_once('=')
            && let Some(apk_path) = apk_part.strip_prefix("package:")
        {
            all_installed.insert(pkg_name.to_string(), apk_path.to_string());
        }
    }
    let output = Command::new("pm")
        .args(["list", "packages", "-3"])
        .output()
        .expect("无法执行 pm list packages");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut user_installed = HashSet::new();
    for line in stdout.lines() {
        if let Some(pkg_name) = line.strip_prefix("package:") {
            user_installed.insert(pkg_name.trim().to_string());
        }
    }
    let mut results = Vec::new();

    for (pkg_name, apk_path) in all_installed {
        let in_global = index.global.packages.contains_key(&pkg_name);
        let in_user = user_installed.contains(&pkg_name);

        if in_global || in_user {
            results.push(PackageMatch {
                pkg_name,
                apk_path,
                is_global: in_global,
            })
        }
    }
    Ok(results)
}

fn get_adapted_packages(uxicons_path: &str) -> Vec<String> {
    let path = Path::new(uxicons_path);
    if !path.exists() {
        return vec![];
    }
    WalkDir::new(path)
        .min_depth(1)
        .max_depth(1)
        .contents_first(false)
        .into_iter()
        .filter_map(|e| {
            let entry = e.ok()?;
            if entry.path().is_dir() {
                Some(entry.file_name().to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect()
}

pub async fn get_packages_list(
    index: &Index,
    storage_root: &Path,
) -> anyhow::Result<Vec<PackageInfo>> {
    let mut adapted_pkgs = HashSet::new();
    if storage_root.exists() {
        let mut entries = fs::read_dir(storage_root).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir()
                && let Some(name) = entry.file_name().to_str() {
                    adapted_pkgs.insert(name.to_string());
                }
        }
    }

    let pkgs = get_installed_packages_with_global(index).await?;
    let mut results: Vec<PackageInfo> = Vec::new();

    for pkg in pkgs {
        results.push(PackageInfo {
            package_name: pkg.pkg_name.clone(),
            is_adapted: adapted_pkgs.contains(&pkg.pkg_name),
        })
    }

    Ok(results)
}

pub fn run(uxicons_path: &str, json_mode: bool) {
    let installed_packages = get_installed_packages();
    let adapted_packages = get_adapted_packages(uxicons_path);

    let apps_info: Vec<AppInfo> = installed_packages
        .par_iter()
        .map(|(pkg_name, apk_path)| {
            let is_monet_supported = monet_scan::check_monet(apk_path).is_some();
            let is_adapted = adapted_packages.contains(pkg_name);
            AppInfo {
                name: Cow::Borrowed(pkg_name).into_owned(),
                is_monet_supported_natively: is_monet_supported,
                is_adapted,
                light_icon: None,
                dark_icon: None,
                monet_icon: None,
                mat_icon: None,
            }
        })
        .collect();

    if json_mode {
        match serde_json::to_string_pretty(&apps_info) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("序列化 JSON 出错: {}", e),
        }
    } else {
        println!("软件总数: {}", installed_packages.len());
        println!("已适配数: {}", adapted_packages.len());
        println!("软件列表:");
        for app in &apps_info {
            println!(
                "  {} | 原生支持 Monet: {} | 已适配: {}",
                app.name, app.is_monet_supported_natively, app.is_adapted
            );
        }
    }
}
