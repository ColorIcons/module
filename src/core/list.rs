use crate::utils::monet_scan;
use serde::Serialize;
use std::{borrow::Cow, collections::HashMap, path::Path, process::Command};
use walkdir::WalkDir;
use rayon::prelude::*;

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

fn get_installed_packages() -> HashMap<String, String> {
    let output = Command::new("pm")
        .args(["list", "packages", "-3", "-f"])
        .output()
        .expect("无法执行 pm list packages");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut map = HashMap::new();
    for line in stdout.lines() {
        if let Some((apk_part, pkg_name)) = line.rsplit_once('=') 
            && let Some(apk_path) = apk_part.strip_prefix("package:") 
        {
            map.insert(pkg_name.to_string(), apk_path.to_string());
        }
    }
    map
}

fn get_adapted_packages(uxicons_path: &str) -> Vec<String> {
    let path = Path::new(uxicons_path);
    if !path.exists() { return vec![]; }
    WalkDir::new(path)
        .min_depth(1)
        .max_depth(1)
        .contents_first(false)
        .into_iter()
        .filter_map(|e| {
            let entry = e.ok()?;
            if entry.path().is_dir() {
                Some(entry.file_name().to_string_lossy().to_string())
            } else { None }
        })
        .collect()
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
