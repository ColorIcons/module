use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Index {
    pub repo_version: u32,
    pub generated_at: u64,
    pub global: GlobalIndex,
    pub packages: HashMap<String, PackageInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalIndex {
    pub version: String,
    pub files: HashMap<String, FileInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageInfo {
    pub version: String,
    pub manifest: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ManifestFile {
    pub file: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub required: bool,
    pub sha256: Option<String>,
    pub size: Option<u64>,
    pub variant: Option<String>,
}
