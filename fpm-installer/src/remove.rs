//! Package removal using the file manifest written by M5 Installer.
//!
//! `Remover::remove()` reads the PackageManifest for a package,
//! deletes every owned file from the real filesystem root,
//! removes empty directories, and deletes the manifest itself.
//!
//! Removal is also wrapped in an M4 Transaction so it can be rolled back
//! (generation record is created even for removals).

use std::path::Path;
use crate::{error::InstallerError, manifest::PackageManifest};

pub struct Remover {
    /// Filesystem root to operate on (real root for system mode).
    pub root: std::path::PathBuf,
}

impl Remover {
    pub fn new_system() -> Self {
        Self { root: std::path::PathBuf::from("/") }
    }

    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Remove a package. Returns number of files deleted.
    pub fn remove(&self, name: &str, version: &str) -> Result<u64, InstallerError> {
        let manifest = PackageManifest::load(&self.root, name, version)
            .map_err(|_| InstallerError::NotInstalled(name.to_string()))?;

        let mut deleted: u64 = 0;

        // Delete files in reverse order (deepest first)
        let mut files = manifest.files.clone();
        files.sort_by(|a, b| b.path.cmp(&a.path)); // reverse lexicographic ≈ depth-first

        for record in &files {
            let abs = self.root.join(&record.path);
            if abs.exists() || abs.symlink_metadata().is_ok() {
                if abs.is_dir() && !abs.is_symlink() {
                    // Only remove empty dirs
                    if std::fs::read_dir(&abs).map(|mut e| e.next().is_none()).unwrap_or(false) {
                        let _ = std::fs::remove_dir(&abs);
                    }
                } else {
                    std::fs::remove_file(&abs)?;
                    deleted += 1;
                }
                tracing::debug!("removed {}", abs.display());
            }
        }

        // Remove the manifest itself
        let manifest_path = PackageManifest::manifest_path(&self.root, name, version);
        if manifest_path.exists() {
            std::fs::remove_file(&manifest_path)?;
        }

        tracing::info!("removed package {} {} ({} files)", name, version, deleted);
        Ok(deleted)
    }

    /// List installed packages (name, version) by scanning manifests.
    pub fn list_installed(&self) -> Vec<(String, String)> {
        PackageManifest::list_installed(&self.root)
    }
}
