//! DbSync: reads on-disk artefacts from M4 (GenerationMeta) and
//! M5 (PackageManifest) and writes them into the Database.
//!
//! Called by fpmd after every `trx.commit()` to keep the DB in sync
//! with the on-disk generation state.

use fpm_core::{
    generation::Generation,
    paths::FpmPaths,
};
use fpm_installer::manifest::PackageManifest;

use crate::{
    db::Database,
    error::DbError,
    models::DbFile,
};

pub struct DbSync<'a> {
    pub db: &'a Database,
    pub paths: &'a FpmPaths,
}

impl<'a> DbSync<'a> {
    pub fn new(db: &'a Database, paths: &'a FpmPaths) -> Self {
        Self { db, paths }
    }

    /// Sync a single generation after it has been committed.
    ///
    /// 1. Insert the generation record.
    /// 2. For each package in the generation's package list:
    ///    a. Insert/update the packages row.
    ///    b. Load its PackageManifest from disk.
    ///    c. Insert all file records.
    pub fn sync_generation(
        &self,
        gen_id: u64,
        explicit_names: &[String], // packages the user explicitly requested
    ) -> Result<(), DbError> {
        let gen = Generation::load(self.paths, gen_id)
            .map_err(|e| DbError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("generation {} not found: {}", gen_id, e),
            )))?;

        let packages_json = serde_json::to_string(&gen.meta.packages)?;
        let created_at = gen.meta.created_at.to_rfc3339();

        self.db.insert_generation(
            gen_id as i64,
            &gen.meta.description,
            &created_at,
            gen.meta.parent.map(|p| p as i64),
            &packages_json,
        )?;

        // Root for this generation's installed files.
        // In the current design the generation directory contains a `root/`
        // subtree after commit.
        let root = gen.path.join("root");

        for pkg in &gen.meta.packages {
            let explicit = explicit_names.iter().any(|n| n == &pkg.name);
            let manifest = PackageManifest::load(&root, &pkg.name, &pkg.version);

            let (installed_size, db_files) = match manifest {
                Ok(m) => {
                    let size: i64 = m.files.iter().map(|f| f.size as i64).sum();
                    let files: Vec<DbFile> = m.files.iter().map(|f| DbFile {
                        id: 0,
                        package_id: 0, // filled by insert_files
                        package_name: pkg.name.clone(),
                        path: f.path.clone(),
                        blake3: f.blake3.clone(),
                        size: f.size as i64,
                        file_type: format!("{:?}", f.file_type).to_lowercase(),
                    }).collect();
                    (size, files)
                }
                Err(_) => (0, vec![]),
            };

            let pkg_id = self.db.insert_package(
                &pkg.name,
                &pkg.version,
                gen_id as i64,
                pkg.blake3.as_deref(),
                installed_size,
                explicit,
            )?;

            if !db_files.is_empty() {
                self.db.insert_files(pkg_id, &pkg.name, &db_files)?;
            }
        }

        tracing::info!(
            "db: synced generation {} ({} packages)",
            gen_id,
            gen.meta.packages.len()
        );
        Ok(())
    }

    /// Full re-sync: iterate all on-disk generations and sync each one.
    /// Used on first startup or after a manual filesystem change.
    pub fn full_resync(&self) -> Result<usize, DbError> {
        let ids = Generation::list_ids(self.paths)
            .map_err(|e| DbError::Io(std::io::Error::new(
                std::io::ErrorKind::Other, e.to_string()
            )))?;
        let mut synced = 0;
        for id in &ids {
            self.sync_generation(*id, &[])?;
            synced += 1;
        }
        tracing::info!("db: full resync complete ({} generations)", synced);
        Ok(synced)
    }
}
