//! Main installer orchestrator — M5 entry point.
//!
//! `Installer::install_plan()` takes an M4 `Transaction` + `InstallPlan` and:
//!   1. Iterates actionable entries (Install / Upgrade / Reinstall)
//!   2. For each entry:
//!      a. run_pre_install hook
//!      b. extract DATA/ into trx.root_dir()
//!      c. run layout fixups (ldconfig, .desktop)
//!      d. write PackageManifest
//!      e. run_post_install hook
//!   3. Returns InstallResult summarising what was done
//!
//! The caller is responsible for calling trx.commit() or trx.abort()
//! based on whether install_plan() succeeds.

use std::collections::HashMap;
use std::path::PathBuf;

use fpm_core::{
    plan::{InstallPlan, PlanEntry, PlanOp},
    trx::Transaction,
};

use crate::{
    error::InstallerError,
    extract::extract_data,
    hooks::{run_post_install, run_pre_install},
    layout::run_layout_fixups,
    manifest::PackageManifest,
};

/// Summary of one completed install operation.
#[derive(Debug, Clone)]
pub struct PackageInstallRecord {
    pub name: String,
    pub version: String,
    pub op: String,
    /// Files written into the staging root
    pub files_installed: u64,
    /// Total bytes written
    pub bytes_installed: u64,
}

/// Result returned to the caller after install_plan() completes.
#[derive(Debug, Clone)]
pub struct InstallResult {
    pub records: Vec<PackageInstallRecord>,
    /// Packages skipped (already at correct version)
    pub skipped: Vec<String>,
}

impl InstallResult {
    pub fn total_files(&self) -> u64 {
        self.records.iter().map(|r| r.files_installed).sum()
    }
    pub fn total_bytes(&self) -> u64 {
        self.records.iter().map(|r| r.bytes_installed).sum()
    }
}

pub struct Installer {
    /// If true, run pre/post hooks. Can be disabled for tests.
    pub run_hooks: bool,
    /// If true, check for file ownership conflicts before extracting.
    pub check_conflicts: bool,
}

impl Default for Installer {
    fn default() -> Self {
        Self { run_hooks: true, check_conflicts: true }
    }
}

impl Installer {
    pub fn new() -> Self { Self::default() }

    pub fn without_hooks() -> Self {
        Self { run_hooks: false, check_conflicts: false }
    }

    /// Core install routine. Writes files into `trx.root_dir()`.
    /// Does NOT call trx.commit() — caller does that after inspecting the result.
    pub fn install_plan(
        &self,
        trx: &Transaction,
        plan: &InstallPlan,
    ) -> Result<InstallResult, InstallerError> {
        let root = trx.root_dir();
        std::fs::create_dir_all(&root)?;

        let mut records = vec![];
        let mut skipped = vec![];
        // ownership map: rel_path → package name (for conflict detection)
        let mut owned: HashMap<String, String> = HashMap::new();

        for entry in &plan.entries {
            match &entry.op {
                PlanOp::AlreadyInstalled => {
                    skipped.push(entry.name.clone());
                    continue;
                }
                _ => {}
            }

            let record = self.install_one(entry, &root, &mut owned)?;
            records.push(record);
        }

        Ok(InstallResult { records, skipped })
    }

    fn install_one(
        &self,
        entry: &PlanEntry,
        root: &PathBuf,
        owned: &mut HashMap<String, String>,
    ) -> Result<PackageInstallRecord, InstallerError> {
        let fpkg_path = entry.fpkg_path.as_ref()
            .ok_or_else(|| InstallerError::MissingFpkgPath(entry.name.clone()))?;

        tracing::info!(
            "[{:?}] {} {}",
            entry.op, entry.name, entry.version
        );

        // 1. Pre-install hook
        if self.run_hooks {
            run_pre_install(root, &entry.name, &entry.version)?;
        }

        // 2. Extract DATA/
        let extracted = extract_data(fpkg_path, root)?;

        // 3. Conflict check
        if self.check_conflicts {
            for f in &extracted {
                let rel = f.rel_path.to_string_lossy().into_owned();
                if let Some(owner) = owned.get(&rel) {
                    if owner != &entry.name {
                        return Err(InstallerError::FileConflict {
                            path: rel,
                            owner: owner.clone(),
                        });
                    }
                }
            }
        }

        // Register ownership
        for f in &extracted {
            owned.insert(
                f.rel_path.to_string_lossy().into_owned(),
                entry.name.clone(),
            );
        }

        // 4. Layout fixups (ldconfig, .desktop)
        run_layout_fixups(root)?;

        // 5. Write file manifest
        let mut manifest = PackageManifest::new(&entry.name, &entry.version);
        manifest.add_from_extracted(&extracted);
        manifest.save(root)?;

        // 6. Post-install hook
        if self.run_hooks {
            run_post_install(root, &entry.name, &entry.version)?;
        }

        let files_installed = extracted.len() as u64;
        let bytes_installed: u64 = extracted.iter().map(|f| f.size_bytes).sum();

        tracing::info!(
            "{} {}: {} files, {} bytes",
            entry.name, entry.version, files_installed, bytes_installed
        );

        Ok(PackageInstallRecord {
            name: entry.name.clone(),
            version: entry.version.clone(),
            op: format!("{:?}", entry.op),
            files_installed,
            bytes_installed,
        })
    }
}
