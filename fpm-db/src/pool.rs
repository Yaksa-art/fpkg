use std::path::Path;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OpenFlags;

use crate::error::DbError;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn open_pool(path: &Path, max_size: u32) -> Result<DbPool, DbError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let manager = SqliteConnectionManager::file(path)
        .with_flags(
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_init(|conn| {
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA foreign_keys = ON;
                 PRAGMA busy_timeout = 5000;",
            )
        });

    let pool = Pool::builder()
        .max_size(max_size)
        .build(manager)
        .map_err(|e| DbError::Pool(e.to_string()))?;

    {
        let conn = pool.get().map_err(|e| DbError::Pool(e.to_string()))?;
        migrate(&conn)?;
    }

    Ok(pool)
}

pub fn open_pool_in_memory() -> Result<DbPool, DbError> {
    let manager = SqliteConnectionManager::memory().with_init(|conn| {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )
    });

    let pool = Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|e| DbError::Pool(e.to_string()))?;

    {
        let conn = pool.get().map_err(|e| DbError::Pool(e.to_string()))?;
        migrate(&conn)?;
    }

    Ok(pool)
}

fn migrate(conn: &rusqlite::Connection) -> Result<(), DbError> {
    conn.execute_batch("
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

        CREATE TABLE IF NOT EXISTS repos (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT    NOT NULL UNIQUE,
            url             TEXT    NOT NULL,
            priority        INTEGER NOT NULL DEFAULT 100,
            enabled         INTEGER NOT NULL DEFAULT 1,
            last_synced_at  TEXT,
            etag            TEXT,
            pubkey_path     TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_files_package_id ON files(package_id);
        CREATE INDEX IF NOT EXISTS idx_files_path       ON files(path);
        CREATE INDEX IF NOT EXISTS idx_packages_name    ON packages(name);
        CREATE INDEX IF NOT EXISTS idx_packages_gen     ON packages(generation_id);
        CREATE INDEX IF NOT EXISTS idx_repos_priority   ON repos(priority DESC);
    ")?;
    Ok(())
}
