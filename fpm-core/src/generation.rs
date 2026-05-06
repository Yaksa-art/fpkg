use crate::{error::TrxError, paths::FpmPaths};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub type GenerationId = u64;

/// Metadata stored in generations/<id>/meta.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMeta {
    pub id: GenerationId,
    pub created_at: DateTime<Utc>,
    /// Human description: "install firefox 125.0.3", "remove vim", "rollback"
    pub description: String,
    /// Packages installed in this generation (name + version)
    pub packages: Vec<GenerationPackage>,
    /// Generation this was derived from (None = first generation)
    pub parent: Option<GenerationId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationPackage {
    pub name: String,
    pub version: String,
    /// BLAKE3 hash of the .fpkg that was installed
    pub blake3: Option<String>,
}

/// A resolved, on-disk generation.
#[derive(Debug, Clone)]
pub struct Generation {
    pub meta: GenerationMeta,
    /// Path: lib_dir/generations/<id>/
    pub path: PathBuf,
}

impl Generation {
    /// Load a generation from disk by id.
    pub fn load(paths: &FpmPaths, id: GenerationId) -> Result<Self, TrxError> {
        let dir = paths.generation_dir(id);
        if !dir.exists() {
            return Err(TrxError::GenerationNotFound(id));
        }
        let meta_path = dir.join("meta.json");
        let raw = std::fs::read_to_string(&meta_path)?;
        let meta: GenerationMeta = serde_json::from_str(&raw)?;
        Ok(Self { meta, path: dir })
    }

    /// List all generation ids present on disk, sorted ascending.
    pub fn list_ids(paths: &FpmPaths) -> Result<Vec<GenerationId>, TrxError> {
        let dir = paths.generations_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut ids = vec![];
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if let Ok(name) = entry.file_name().into_string() {
                if let Ok(id) = name.parse::<GenerationId>() {
                    ids.push(id);
                }
            }
        }
        ids.sort_unstable();
        Ok(ids)
    }

    /// Return the current (active) generation id via the `current` symlink.
    pub fn current_id(paths: &FpmPaths) -> Result<GenerationId, TrxError> {
        let link = paths.current_link();
        if !link.exists() {
            return Err(TrxError::NoCurrentGeneration);
        }
        let target = std::fs::read_link(&link)?;
        target
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|s| s.parse::<GenerationId>().ok())
            .ok_or(TrxError::NoCurrentGeneration)
    }

    /// Return the next generation id (max existing + 1, or 1 if none).
    pub fn next_id(paths: &FpmPaths) -> Result<GenerationId, TrxError> {
        let ids = Self::list_ids(paths)?;
        Ok(ids.last().copied().unwrap_or(0) + 1)
    }

    /// Persist meta.json into the generation directory.
    pub fn save(paths: &FpmPaths, meta: &GenerationMeta) -> Result<PathBuf, TrxError> {
        let dir = paths.generation_dir(meta.id);
        std::fs::create_dir_all(&dir)?;
        let json = serde_json::to_string_pretty(meta)?;
        std::fs::write(dir.join("meta.json"), json)?;
        Ok(dir)
    }

    /// Prune old generations, keeping `keep` most recent.
    pub fn prune(paths: &FpmPaths, keep: usize) -> Result<Vec<GenerationId>, TrxError> {
        let mut ids = Self::list_ids(paths)?;
        let current = Self::current_id(paths).unwrap_or(0);
        // Never prune the current generation
        ids.retain(|&id| id != current);
        if ids.len() <= keep {
            return Ok(vec![]);
        }
        let to_remove = ids[..ids.len() - keep].to_vec();
        for id in &to_remove {
            let dir = paths.generation_dir(*id);
            tracing::info!("Pruning generation {}", id);
            let _ = std::fs::remove_dir_all(&dir);
        }
        Ok(to_remove)
    }
}
