use std::path::{Path, PathBuf};
use crate::error::FetchError;

/// Manages the local package cache at cache_dir/<name>-<version>.fpkg
pub struct PackageCache {
    pub root: PathBuf,
}

impl PackageCache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn package_path(&self, name: &str, version: &str) -> PathBuf {
        self.root.join(format!("{}-{}.fpkg", name, version))
    }

    pub fn partial_path(&self, name: &str, version: &str) -> PathBuf {
        self.root.join(format!("{}-{}.fpkg.part", name, version))
    }

    pub fn etag_path(&self, name: &str, version: &str) -> PathBuf {
        self.root.join(format!("{}-{}.etag", name, version))
    }

    /// Returns true if a complete cached copy exists and its BLAKE3 matches.
    pub fn is_cached(&self, name: &str, version: &str, expected_blake3: Option<&str>) -> bool {
        let path = self.package_path(name, version);
        if !path.exists() {
            return false;
        }
        if let Some(expected) = expected_blake3 {
            match Self::blake3_file(&path) {
                Ok(actual) => actual == expected,
                Err(_) => false,
            }
        } else {
            true
        }
    }

    /// Atomically promote .part file to final path.
    pub fn commit_partial(&self, name: &str, version: &str) -> Result<PathBuf, FetchError> {
        let partial = self.partial_path(name, version);
        let final_path = self.package_path(name, version);
        std::fs::rename(&partial, &final_path)?;
        Ok(final_path)
    }

    pub fn partial_size(&self, name: &str, version: &str) -> u64 {
        self.partial_path(name, version)
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0)
    }

    pub fn ensure_dir(&self) -> Result<(), FetchError> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }

    pub fn read_etag(&self, name: &str, version: &str) -> Option<String> {
        std::fs::read_to_string(self.etag_path(name, version)).ok()
    }

    pub fn write_etag(&self, name: &str, version: &str, etag: &str) {
        let _ = std::fs::write(self.etag_path(name, version), etag);
    }

    fn blake3_file(path: &Path) -> Result<String, FetchError> {
        let data = std::fs::read(path)?;
        let hash = blake3::hash(&data);
        Ok(hex::encode(hash.as_bytes()))
    }
}
