use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{error::DbError, pool::DbPool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbRepo {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub priority: i64,
    pub enabled: bool,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub etag: Option<String>,
    pub pubkey_path: Option<String>,
}

pub struct RepoStore<'a> {
    pool: &'a DbPool,
}

impl<'a> RepoStore<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    fn conn(&self) -> Result<r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>, DbError> {
        self.pool.get().map_err(|e| DbError::Pool(e.to_string()))
    }

    pub fn upsert(
        &self,
        name: &str,
        url: &str,
        priority: i64,
        enabled: bool,
        pubkey_path: Option<&str>,
    ) -> Result<i64, DbError> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO repos (name, url, priority, enabled, pubkey_path)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(name) DO UPDATE SET
               url         = excluded.url,
               priority    = excluded.priority,
               enabled     = excluded.enabled,
               pubkey_path = excluded.pubkey_path",
            params![name, url, priority, enabled as i64, pubkey_path],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn mark_synced(&self, name: &str, etag: Option<&str>) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE repos SET last_synced_at = ?1, etag = ?2 WHERE name = ?3",
            params![now, etag, name],
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<DbRepo>, DbError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, url, priority, enabled, last_synced_at, etag, pubkey_path
             FROM repos ORDER BY priority DESC, name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DbRepo {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                priority: row.get(3)?,
                enabled: row.get::<_, i64>(4)? != 0,
                last_synced_at: {
                    let s: Option<String> = row.get(5)?;
                    s.and_then(|s| s.parse().ok())
                },
                etag: row.get(6)?,
                pubkey_path: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn list_enabled(&self) -> Result<Vec<DbRepo>, DbError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, url, priority, enabled, last_synced_at, etag, pubkey_path
             FROM repos WHERE enabled = 1 ORDER BY priority DESC, name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DbRepo {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                priority: row.get(3)?,
                enabled: row.get::<_, i64>(4)? != 0,
                last_synced_at: {
                    let s: Option<String> = row.get(5)?;
                    s.and_then(|s| s.parse().ok())
                },
                etag: row.get(6)?,
                pubkey_path: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn get(&self, name: &str) -> Result<Option<DbRepo>, DbError> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT id, name, url, priority, enabled, last_synced_at, etag, pubkey_path
             FROM repos WHERE name = ?1",
            params![name],
            |row| {
                Ok(DbRepo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    url: row.get(2)?,
                    priority: row.get(3)?,
                    enabled: row.get::<_, i64>(4)? != 0,
                    last_synced_at: {
                        let s: Option<String> = row.get(5)?;
                        s.and_then(|s| s.parse().ok())
                    },
                    etag: row.get(6)?,
                    pubkey_path: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(DbError::from)
    }

    pub fn set_enabled(&self, name: &str, enabled: bool) -> Result<bool, DbError> {
        let conn = self.conn()?;
        let n = conn.execute(
            "UPDATE repos SET enabled = ?1 WHERE name = ?2",
            params![enabled as i64, name],
        )?;
        Ok(n > 0)
    }

    pub fn remove(&self, name: &str) -> Result<bool, DbError> {
        let conn = self.conn()?;
        let n = conn.execute("DELETE FROM repos WHERE name = ?1", params![name])?;
        Ok(n > 0)
    }
}
