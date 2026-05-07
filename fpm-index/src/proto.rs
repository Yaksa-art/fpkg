use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexPackage {
    pub name: String,
    pub version: String,
    pub deps: Vec<IndexDep>,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    pub blake3: String,
    pub size: u64,
    pub url_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDep {
    pub name: String,
    #[serde(default)]
    pub version_req: String,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoIndex {
    pub repo: String,
    pub generated_at: String,
    pub packages: Vec<IndexPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaEntry {
    pub op: DeltaOp,
    pub package: IndexPackage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeltaOp {
    Add,
    Remove,
    Update,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoDelta {
    pub repo: String,
    pub base_etag: String,
    pub entries: Vec<DeltaEntry>,
}
