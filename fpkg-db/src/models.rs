use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: i64,
    pub name: String,
    pub version: String,
    pub release: u32,
    pub arch: String,
    pub mode: String,
    pub summary: String,
    pub license: String,
    pub maintainer: String,
    pub homepage: String,
    pub install_size: i64,
    pub origin_format: String,
    pub install_date: String,
    pub manifest_hash: String,
    pub content_tree: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPackage {
    pub name: String,
    pub version: String,
    pub release: u32,
    pub arch: String,
    pub mode: String,
    pub summary: String,
    pub license: String,
    pub maintainer: String,
    pub homepage: String,
    pub install_size: i64,
    pub origin_format: String,
    pub install_date: String,
    pub manifest_hash: String,
    pub content_tree: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFile {
    pub id: i64,
    pub package_id: i64,
    pub path: String,
    pub hash: String,
    pub size: i64,
    pub is_config: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFile {
    pub path: String,
    pub hash: String,
    pub size: i64,
    pub is_config: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generation {
    pub id: i64,
    pub created_at: String,
    pub action: String,
    pub packages: Vec<GenerationEntry>,
    pub note: String,
    pub rolled_back: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationEntry {
    pub name: String,
    pub version: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub repo_type: String,
    pub enabled: bool,
    pub priority: i64,
    pub pubkey: String,
    pub last_sync: String,
    pub suite: String,
    pub components: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRepo {
    pub name: String,
    pub url: String,
    pub repo_type: String,
    pub enabled: bool,
    pub priority: i64,
    pub pubkey: String,
    pub suite: String,
    pub components: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hold {
    pub id: i64,
    pub name: String,
    pub version: String,
    pub reason: String,
}
