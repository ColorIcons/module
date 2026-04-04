use std::path::PathBuf;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static STORAGE_ROOT: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from("/data/adb/modules/ColorOSIconsPatch/uxicons"));

pub static CONFIG_PATH: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from("/data/adb/ColorOSIconsPatch/config.toml"));

pub static INDEX_FILE_PATH: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from("/data/adb/ColorOSIconsPatch/index.json"));

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub icons: Icons,
    pub network: Network,
    pub repo: Repo,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
pub struct Icons {
    pub light: bool,
    pub dark: bool,
    pub mat: bool,
    pub monochrome: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Network {
    pub concurrency: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Repo {
    pub base_url: String,
}
