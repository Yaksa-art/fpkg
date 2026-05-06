//! Per-package file manifest — the record of every file owned by a package.
//!
//! Stored at:
//!   <staging_root>/var/lib/fpm/manifests/<name>-<version>.json
//!
//! Format (JSON array of FileRecord):
//! [
//!   { "path": "usr/bin/firefox", "blake3": "abc...", "size": 12345, "type": "file" },
//!   { "path": "usr/share/applications/firefox.desktop", "blake3": "", "size": 0, "type": "file" },
//!   ...
//! ]
//!
//! Used by M5 Remove to know which files to delete,
//! and by M8 Database to record ownership.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::{error::InstallerError, extract::ExtractedFile};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    File,
    Symlink,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    /// Relative to filesystem root (no leading /)
    pub path: String,
    pub blake3: String,
    pub size: u64,
    #[serde(rename = "type")]
    pub file_type: FileType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub files: Vec<FileRecord>,
}

impl PackageManifest {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self { name: name.into(), version: version.into(), files: vec![] }
    }

    pub fn add_from_extracted(&mut self, files: &[ExtractedFile]) {
        for f in files {
            self.files.push(FileRecord {
                path: f.rel_path.to_string_lossy().into_owned(),
                blake3: f.blake3.clone(),
                size: f.size_bytes,
                file_type: if f.blake3.is_empty() { FileType::Symlink } else { FileType::File },
            });
        }
    }

    /// Manifest file path inside the staging root.
    pub fn manifest_path(root: &Path, name: &str, version: &str) -> PathBuf {
        root.join("var/lib/fpm/manifests")
            .join(format!("{}-{}.json", name, version))
    }

    /// Save manifest JSON into the staging root.
    pub fn save(&self, root: &Path) -> Result<PathBuf, InstallerError> {
        let path = Self::manifest_path(root, &self.name, &self.version);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json).map_err(|e| {
            InstallerError::ManifestWrite(self.name.clone(), e.to_string())
        })?;
        tracing::debug!("manifest saved: {}", path.display());
        Ok(path)
    }

    /// Load a manifest from an installed root.
    pub fn load(root: &Path, name: &str, version: &str) -> Result<Self, InstallerError> {
        let path = Self::manifest_path(root, name, version);
        let raw = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    /// List all installed package manifests in a root.
    pub fn list_installed(root: &Path) -> Vec<(String, String)> {
        let dir = root.join("var/lib/fpm/manifests");
        if !dir.exists() { return vec![]; }
        walkdir::WalkDir::new(&dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
            .filter_map(|e| {
                let stem = e.path().file_stem()?.to_string_lossy().into_owned();
                // stem format: name-version (version may contain dots + hyphens)
                // Split on last occurrence of a version-like segment
                // Simple heuristic: rfind '-' where right side starts with a digit
                let bytes = stem.as_bytes();
                for i in (1..stem.len()).rev() {
                    if bytes[i - 1] == b'-' && bytes[i].is_ascii_digit() {
                        let name = stem[..i - 1].to_string();
                        let ver  = stem[i..].to_string();
                        return Some((name, ver));
                    }
                }
                None
            })
            .collect()
    }
}
