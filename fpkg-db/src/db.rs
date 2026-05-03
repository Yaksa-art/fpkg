use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

use crate::schema;

pub struct Database {
    pub conn: Connection,
    pub path: PathBuf,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Cannot create directory: {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("Cannot open database: {}", path.display()))?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        schema::migrate(&conn)?;
        Ok(Self { conn, path: path.to_owned() })
    }

    pub fn open_system() -> Result<Self> {
        Self::open(Path::new("/var/lib/fpm/db.sqlite"))
    }

    pub fn open_user() -> Result<Self> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        let path = PathBuf::from(home).join(".local/share/fpm/db.sqlite");
        Self::open(&path)
    }

    pub fn open_default(user_mode: bool) -> Result<Self> {
        if user_mode { Self::open_user() } else { Self::open_system() }
    }

    pub fn stats(&self) -> Result<DbStats> {
        let packages: i64 = self.conn.query_row("SELECT COUNT(*) FROM packages", [], |r| r.get(0))?;
        let files: i64    = self.conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
        let generations: i64 = self.conn.query_row("SELECT COUNT(*) FROM generations", [], |r| r.get(0))?;
        let repos: i64    = self.conn.query_row("SELECT COUNT(*) FROM repos", [], |r| r.get(0))?;
        let holds: i64    = self.conn.query_row("SELECT COUNT(*) FROM hold", [], |r| r.get(0))?;
        Ok(DbStats { packages, files, generations, repos, holds })
    }
}

pub struct DbStats {
    pub packages: i64,
    pub files: i64,
    pub generations: i64,
    pub repos: i64,
    pub holds: i64,
}
