//! Database connection, schema migration, and core CRUD.

use std::path::Path;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    error::DbError,
    models::{DbFile, DbGeneration, DbHold, DbPackage},
};

/// Wrap a rusqlite connection with fpm-specific helpers.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the database at `path`. Runs migrations automatically.
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// In-memory database for tests.
    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    // ------------------------------------------------------------------ schema

    fn migrate(&self) -> Result<(), DbError> {
        self.conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS generations (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                gen_id          INTEGER NOT NULL UNIQUE,
                description     TEXT    NOT NULL,
                created_at      TEXT    NOT NULL,
                parent_gen_id   INTEGER,
                packages_json   TEXT    NOT NULL DEFAULT '[]'
            );

            CREATE TABLE IF NOT EXISTS packages (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                name            TEXT    NOT NULL,
                version         TEXT    NOT NULL,
                generation_id   INTEGER NOT NULL REFERENCES generations(gen_id),
                blake3          TEXT,
                installed_size  INTEGER NOT NULL DEFAULT 0,
                installed_at    TEXT    NOT NULL,
                explicit        INTEGER NOT NULL DEFAULT 1,
                UNIQUE(name, version)
            );

            CREATE TABLE IF NOT EXISTS files (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                package_id      INTEGER NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
                package_name    TEXT    NOT NULL,
                path            TEXT    NOT NULL UNIQUE,
                blake3          TEXT    NOT NULL DEFAULT '',
                size            INTEGER NOT NULL DEFAULT 0,
                file_type       TEXT    NOT NULL DEFAULT 'file'
            );

            CREATE TABLE IF NOT EXISTS holds (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                package_name    TEXT    NOT NULL UNIQUE,
                reason          TEXT,
                held_at         TEXT    NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_files_package_id   ON files(package_id);
            CREATE INDEX IF NOT EXISTS idx_files_path         ON files(path);
            CREATE INDEX IF NOT EXISTS idx_packages_name      ON packages(name);
            CREATE INDEX IF NOT EXISTS idx_packages_gen       ON packages(generation_id);
        ")?;
        Ok(())
    }

    // ---------------------------------------------------------------- packages

    /// Insert a new installed package record.
    pub fn insert_package(
        &self,
        name: &str,
        version: &str,
        generation_id: i64,
        blake3: Option<&str>,
        installed_size: i64,
        explicit: bool,
    ) -> Result<i64, DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO packages \
             (name, version, generation_id, blake3, installed_size, installed_at, explicit) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![name, version, generation_id, blake3, installed_size, now, explicit as i64],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Remove a package and all its file records (cascade).
    pub fn remove_package(&self, name: &str, version: &str) -> Result<bool, DbError> {
        let n = self.conn.execute(
            "DELETE FROM packages WHERE name = ?1 AND version = ?2",
            params![name, version],
        )?;
        Ok(n > 0)
    }

    /// Fetch a package by name (most recently installed version).
    pub fn get_package(&self, name: &str) -> Result<Option<DbPackage>, DbError> {
        self.conn.query_row(
            "SELECT id, name, version, generation_id, blake3, installed_size, installed_at, explicit \
             FROM packages WHERE name = ?1 ORDER BY id DESC LIMIT 1",
            params![name],
            |row| {
                Ok(DbPackage {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    generation_id: row.get(3)?,
                    blake3: row.get(4)?,
                    installed_size: row.get(5)?,
                    installed_at: {
                        let s: String = row.get(6)?;
                        s.parse().unwrap_or(Utc::now())
                    },
                    explicit: row.get::<_, i64>(7)? != 0,
                })
            },
        ).optional().map_err(DbError::from)
    }

    /// List all installed packages.
    pub fn list_packages(&self) -> Result<Vec<DbPackage>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, version, generation_id, blake3, installed_size, installed_at, explicit \
             FROM packages ORDER BY name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DbPackage {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                generation_id: row.get(3)?,
                blake3: row.get(4)?,
                installed_size: row.get(5)?,
                installed_at: {
                    let s: String = row.get(6)?;
                    s.parse().unwrap_or(Utc::now())
                },
                explicit: row.get::<_, i64>(7)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    // ------------------------------------------------------------------- files

    /// Bulk-insert file records for a package.
    pub fn insert_files(
        &self,
        package_id: i64,
        package_name: &str,
        files: &[crate::models::DbFile],
    ) -> Result<(), DbError> {
        for f in files {
            self.conn.execute(
                "INSERT OR REPLACE INTO files \
                 (package_id, package_name, path, blake3, size, file_type) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    package_id, package_name,
                    &f.path, &f.blake3, f.size, &f.file_type
                ],
            )?;
        }
        Ok(())
    }

    /// Which package owns `path`? Returns package name or None.
    pub fn owner_of(&self, path: &str) -> Result<Option<String>, DbError> {
        self.conn.query_row(
            "SELECT package_name FROM files WHERE path = ?1",
            params![path],
            |row| row.get(0),
        ).optional().map_err(DbError::from)
    }

    /// All files belonging to a package.
    pub fn files_of(&self, package_name: &str) -> Result<Vec<DbFile>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, package_id, package_name, path, blake3, size, file_type \
             FROM files WHERE package_name = ?1 ORDER BY path"
        )?;
        let rows = stmt.query_map(params![package_name], |row| {
            Ok(DbFile {
                id: row.get(0)?,
                package_id: row.get(1)?,
                package_name: row.get(2)?,
                path: row.get(3)?,
                blake3: row.get(4)?,
                size: row.get(5)?,
                file_type: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    // ------------------------------------------------------------- generations

    pub fn insert_generation(
        &self,
        gen_id: i64,
        description: &str,
        created_at: &str,
        parent_gen_id: Option<i64>,
        packages_json: &str,
    ) -> Result<i64, DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO generations \
             (gen_id, description, created_at, parent_gen_id, packages_json) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![gen_id, description, created_at, parent_gen_id, packages_json],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_generations(&self) -> Result<Vec<DbGeneration>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, gen_id, description, created_at, parent_gen_id, packages_json \
             FROM generations ORDER BY gen_id"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DbGeneration {
                id: row.get(0)?,
                gen_id: row.get(1)?,
                description: row.get(2)?,
                created_at: {
                    let s: String = row.get(3)?;
                    s.parse().unwrap_or(Utc::now())
                },
                parent_gen_id: row.get(4)?,
                packages_json: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn get_generation(&self, gen_id: i64) -> Result<Option<DbGeneration>, DbError> {
        self.conn.query_row(
            "SELECT id, gen_id, description, created_at, parent_gen_id, packages_json \
             FROM generations WHERE gen_id = ?1",
            params![gen_id],
            |row| Ok(DbGeneration {
                id: row.get(0)?,
                gen_id: row.get(1)?,
                description: row.get(2)?,
                created_at: { let s: String = row.get(3)?; s.parse().unwrap_or(Utc::now()) },
                parent_gen_id: row.get(4)?,
                packages_json: row.get(5)?,
            }),
        ).optional().map_err(DbError::from)
    }

    // --------------------------------------------------------------- holds

    pub fn hold(&self, package_name: &str, reason: Option<&str>) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO holds (package_name, reason, held_at) VALUES (?1, ?2, ?3)",
            params![package_name, reason, now],
        )?;
        Ok(())
    }

    pub fn unhold(&self, package_name: &str) -> Result<bool, DbError> {
        let n = self.conn.execute(
            "DELETE FROM holds WHERE package_name = ?1",
            params![package_name],
        )?;
        Ok(n > 0)
    }

    pub fn is_held(&self, package_name: &str) -> Result<bool, DbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM holds WHERE package_name = ?1",
            params![package_name],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn list_holds(&self) -> Result<Vec<DbHold>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, package_name, reason, held_at FROM holds ORDER BY package_name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DbHold {
                id: row.get(0)?,
                package_name: row.get(1)?,
                reason: row.get(2)?,
                held_at: { let s: String = row.get(3)?; s.parse().unwrap_or(Utc::now()) },
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    // ----------------------------------------------------------------- helpers

    /// Total installed size across all packages (bytes).
    pub fn total_installed_size(&self) -> Result<i64, DbError> {
        let s: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(installed_size), 0) FROM packages",
            [],
            |row| row.get(0),
        )?;
        Ok(s)
    }

    /// Number of installed packages.
    pub fn package_count(&self) -> Result<i64, DbError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM packages", [], |row| row.get(0)
        )?;
        Ok(n)
    }
}
