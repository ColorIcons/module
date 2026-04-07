use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::model::Icons;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Index {
    pub repo_version: u32,
    pub generated_at: u64,
    pub global: GlobalIndex,
    pub packages: HashMap<String, PackageInfo>,
    #[serde(default)]
    pub icons: Icons,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GlobalIndex {
    pub version: String,
    pub files: HashMap<String, FileInfo>,
    pub packages: HashMap<String, PackageInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileInfo {
    pub sha256: String,
    pub size: u64,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub file_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PackageInfo {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    pub files: HashMap<String, FileInfo>,
}
