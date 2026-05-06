use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("hash mismatch for {path}: expected {expected}, got {actual}")]
    HashMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("download failed for {url}: {source}")]
    Download {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("no mirrors available for package {0}")]
    NoMirrors(String),
    #[error("all mirrors failed for package {0}")]
    AllMirrorsFailed(String),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PackageUrl {
    pub name: String,
    pub version: String,
    pub urls: Vec<String>,
    pub blake3: Option<String>,
    pub size: Option<u64>,
}

impl PackageUrl {
    pub fn cache_key(&self) -> String {
        format!("{}-{}", self.name, self.version)
    }
}
