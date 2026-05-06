//! M4 Transaction Manager
//!
//! Lifecycle of one fpm operation:
//!
//!   1. `TransactionManager::begin(description)` — allocates next generation id,
//!      creates `pending/` directory, returns a `Transaction` handle.
//!
//!   2. Caller fills the Transaction with PlanEntries (from M1+M2).
//!
//!   3. M5 Installer (upcoming) unpacks DATA/ files into `pending/root/`.
//!
//!   4. `Transaction::commit()` — atomic rename `pending/ → generations/<id>/`,
//!      updates the `current` symlink.
//!
//!   5. On any error: `Transaction::abort()` — removes `pending/`.
//!
//!   Rollback:
//!   `TransactionManager::rollback(id)` — sets `current` symlink to a past
//!   generation, writes a new generation record describing the rollback.

use std::path::PathBuf;

use chrono::Utc;

use crate::{
    error::TrxError,
    generation::{Generation, GenerationId, GenerationMeta, GenerationPackage},
    paths::FpmPaths,
    plan::InstallPlan,
};

/// A live transaction (not yet committed).
#[derive(Debug)]
pub struct Transaction {
    pub id: GenerationId,
    pub description: String,
    pub plan: Option<InstallPlan>,
    pub parent: Option<GenerationId>,
    /// Path to the staging area: lib_dir/pending/
    pub pending_dir: PathBuf,
    paths: FpmPaths,
}

impl Transaction {
    /// Attach a resolved install plan to this transaction.
    pub fn set_plan(&mut self, plan: InstallPlan) {
        self.plan = Some(plan);
    }

    /// Path where M5 Installer should unpack DATA/ files.
    pub fn root_dir(&self) -> PathBuf {
        self.pending_dir.join("root")
    }

    /// Commit: atomically promote pending → generations/<id>, update symlink.
    pub fn commit(self) -> Result<GenerationId, TrxError> {
        if !self.pending_dir.exists() {
            return Err(TrxError::PendingDirMissing(
                self.pending_dir.display().to_string(),
            ));
        }

        // Build package list from the plan
        let packages = self
            .plan
            .as_ref()
            .map(|p| {
                p.entries
                    .iter()
                    .map(|e| GenerationPackage {
                        name: e.name.clone(),
                        version: e.version.clone(),
                        blake3: e.blake3.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let meta = GenerationMeta {
            id: self.id,
            created_at: Utc::now(),
            description: self.description.clone(),
            packages,
            parent: self.parent,
        };

        // Write meta.json into pending/
        let meta_json = serde_json::to_string_pretty(&meta)?;
        std::fs::write(self.pending_dir.join("meta.json"), &meta_json)?;

        // Atomic rename: pending/ → generations/<id>/
        let target = self.paths.generation_dir(self.id);
        std::fs::rename(&self.pending_dir, &target).map_err(|e| {
            TrxError::AtomicRenameFailed(format!(
                "{} -> {}: {}",
                self.pending_dir.display(),
                target.display(),
                e
            ))
        })?;

        // Update current symlink atomically:
        // write a temp symlink, then rename it over current
        let link = self.paths.current_link();
        let tmp_link = self.paths.lib_dir.join(".current.tmp");
        let _ = std::fs::remove_file(&tmp_link);
        std::os::unix::fs::symlink(target.file_name().unwrap(), &tmp_link)?;
        std::fs::rename(&tmp_link, &link).map_err(|e| {
            TrxError::AtomicRenameFailed(format!(
                "symlink update failed: {}", e
            ))
        })?;

        tracing::info!("Generation {} committed: {}", self.id, self.description);
        Ok(self.id)
    }

    /// Abort: remove pending directory entirely.
    pub fn abort(self) -> Result<(), TrxError> {
        if self.pending_dir.exists() {
            std::fs::remove_dir_all(&self.pending_dir)?;
        }
        tracing::warn!("Transaction {} aborted: {}", self.id, self.description);
        Ok(())
    }
}

/// Manages the generation lifecycle.
pub struct TransactionManager {
    pub paths: FpmPaths,
}

impl TransactionManager {
    pub fn new(paths: FpmPaths) -> Self {
        Self { paths }
    }

    pub fn new_system() -> Self {
        Self::new(FpmPaths::system())
    }

    pub fn new_user() -> Self {
        Self::new(FpmPaths::user())
    }

    /// Begin a new transaction.
    /// Fails if a pending/ directory already exists (another transaction is active).
    pub fn begin(&self, description: impl Into<String>) -> Result<Transaction, TrxError> {
        self.paths.ensure_dirs()?;

        let pending = self.paths.pending_dir();
        if pending.exists() {
            // Try to figure out which generation is pending
            let pending_meta = pending.join("meta.json");
            if let Ok(raw) = std::fs::read_to_string(&pending_meta) {
                if let Ok(meta) = serde_json::from_str::<GenerationMeta>(&raw) {
                    return Err(TrxError::AlreadyActive(meta.id));
                }
            }
            return Err(TrxError::AtomicRenameFailed(
                "pending/ exists but has no meta.json — remove manually".into(),
            ));
        }

        let id = Generation::next_id(&self.paths)?;
        let parent = Generation::current_id(&self.paths).ok();

        // Create pending/ and pending/root/ (where M5 will write files)
        let root = pending.join("root");
        std::fs::create_dir_all(&root)?;

        tracing::info!("Transaction {} started (parent={:?})", id, parent);

        Ok(Transaction {
            id,
            description: description.into(),
            plan: None,
            parent,
            pending_dir: pending,
            paths: self.paths.clone(),
        })
    }

    /// Roll back to a specific generation id.
    ///
    /// This does NOT undo filesystem changes made by M5 (that requires
    /// overlay/CoW — see M11 Sandbox). It updates the `current` pointer
    /// and writes a new generation record documenting the rollback.
    pub fn rollback(&self, target_id: GenerationId) -> Result<GenerationId, TrxError> {
        let current = Generation::current_id(&self.paths)?;
        if current == target_id {
            return Err(TrxError::AlreadyCurrent {
                target: target_id,
                current,
            });
        }

        // Ensure target exists
        let target_gen = Generation::load(&self.paths, target_id)?;

        // Create a new generation record that describes this rollback
        let rollback_id = Generation::next_id(&self.paths)?;
        let packages = target_gen.meta.packages.clone();
        let meta = GenerationMeta {
            id: rollback_id,
            created_at: Utc::now(),
            description: format!("rollback to generation {}", target_id),
            packages,
            parent: Some(current),
        };
        Generation::save(&self.paths, &meta)?;

        // Update current symlink to the new rollback generation
        let new_dir = self.paths.generation_dir(rollback_id);
        let link = self.paths.current_link();
        let tmp_link = self.paths.lib_dir.join(".current.tmp");
        let _ = std::fs::remove_file(&tmp_link);
        std::os::unix::fs::symlink(new_dir.file_name().unwrap(), &tmp_link)?;
        std::fs::rename(&tmp_link, &link).map_err(|e| {
            TrxError::AtomicRenameFailed(format!("rollback symlink failed: {}", e))
        })?;

        tracing::info!(
            "Rolled back from generation {} to {} (new gen {})",
            current, target_id, rollback_id
        );
        Ok(rollback_id)
    }

    /// List all generations (most recent last).
    pub fn list_generations(&self) -> Result<Vec<Generation>, TrxError> {
        let ids = Generation::list_ids(&self.paths)?;
        let mut gens = vec![];
        for id in ids {
            gens.push(Generation::load(&self.paths, id)?);
        }
        Ok(gens)
    }

    /// Current active generation.
    pub fn current_generation(&self) -> Result<Generation, TrxError> {
        let id = Generation::current_id(&self.paths)?;
        Generation::load(&self.paths, id)
    }

    /// Prune old generations, keeping `keep` most recent (never prunes current).
    pub fn prune(&self, keep: usize) -> Result<Vec<GenerationId>, TrxError> {
        Generation::prune(&self.paths, keep)
    }
}
