use anyhow::Result;
use rusqlite::Connection;

pub const SCHEMA_VERSION: u32 = 1;

pub fn migrate(conn: &Connection) -> Result<()> {
    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap_or(0);

    if version >= SCHEMA_VERSION {
        return Ok(());
    }

    conn.execute_batch("PRAGMA journal_mode=WAL;")?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS packages (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT    NOT NULL,
            version         TEXT    NOT NULL,
            release         INTEGER NOT NULL DEFAULT 1,
            arch            TEXT    NOT NULL DEFAULT 'x86_64',
            mode            TEXT    NOT NULL DEFAULT 'system',
            summary         TEXT    NOT NULL DEFAULT '',
            license         TEXT    NOT NULL DEFAULT '',
            maintainer      TEXT    NOT NULL DEFAULT '',
            homepage        TEXT    NOT NULL DEFAULT '',
            install_size    INTEGER NOT NULL DEFAULT 0,
            origin_format   TEXT    NOT NULL DEFAULT 'native',
            install_date    TEXT    NOT NULL,
            manifest_hash   TEXT    NOT NULL DEFAULT '',
            content_tree    TEXT    NOT NULL DEFAULT '',
            UNIQUE(name, mode)
        );

        CREATE TABLE IF NOT EXISTS files (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            package_id  INTEGER NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
            path        TEXT    NOT NULL,
            hash        TEXT    NOT NULL DEFAULT '',
            size        INTEGER NOT NULL DEFAULT 0,
            is_config   INTEGER NOT NULL DEFAULT 0,
            UNIQUE(path, package_id)
        );
        CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);

        CREATE TABLE IF NOT EXISTS generations (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at  TEXT    NOT NULL,
            action      TEXT    NOT NULL,
            packages    TEXT    NOT NULL DEFAULT '[]',
            note        TEXT    NOT NULL DEFAULT '',
            rolled_back INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS repos (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT    NOT NULL UNIQUE,
            url         TEXT    NOT NULL,
            repo_type   TEXT    NOT NULL DEFAULT 'fpkg',
            enabled     INTEGER NOT NULL DEFAULT 1,
            priority    INTEGER NOT NULL DEFAULT 50,
            pubkey      TEXT    NOT NULL DEFAULT '',
            last_sync   TEXT    NOT NULL DEFAULT '',
            suite       TEXT    NOT NULL DEFAULT '',
            components  TEXT    NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS hold (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT    NOT NULL UNIQUE,
            version     TEXT    NOT NULL DEFAULT '',
            reason      TEXT    NOT NULL DEFAULT ''
        );
    ")?;

    conn.execute_batch(&format!("PRAGMA user_version = {};", SCHEMA_VERSION))?;
    Ok(())
}
