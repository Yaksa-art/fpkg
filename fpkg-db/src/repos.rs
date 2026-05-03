use anyhow::Result;
use rusqlite::{params, Connection};

use crate::models::{NewRepo, Repo};

pub fn add(conn: &Connection, repo: &NewRepo) -> Result<i64> {
    conn.execute(
        "INSERT INTO repos (name, url, repo_type, enabled, priority, pubkey, suite, components)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            repo.name, repo.url, repo.repo_type,
            repo.enabled as i64, repo.priority,
            repo.pubkey, repo.suite, repo.components,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn remove(conn: &Connection, name: &str) -> Result<bool> {
    let rows = conn.execute("DELETE FROM repos WHERE name=?1", params![name])?;
    Ok(rows > 0)
}

pub fn enable(conn: &Connection, name: &str, enabled: bool) -> Result<()> {
    conn.execute(
        "UPDATE repos SET enabled=?1 WHERE name=?2",
        params![enabled as i64, name],
    )?;
    Ok(())
}

pub fn update_sync(conn: &Connection, name: &str) -> Result<()> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    conn.execute(
        "UPDATE repos SET last_sync=?1 WHERE name=?2",
        params![now, name],
    )?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<Repo>> {
    let mut stmt = conn.prepare(
        "SELECT id,name,url,repo_type,enabled,priority,pubkey,last_sync,suite,components
         FROM repos ORDER BY priority DESC, name",
    )?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(Repo {
            id:         row.get(0)?,
            name:       row.get(1)?,
            url:        row.get(2)?,
            repo_type:  row.get(3)?,
            enabled:    row.get::<_, i64>(4)? != 0,
            priority:   row.get(5)?,
            pubkey:     row.get(6)?,
            last_sync:  row.get(7)?,
            suite:      row.get(8)?,
            components: row.get(9)?,
        });
    }
    Ok(out)
}

pub fn get_by_name(conn: &Connection, name: &str) -> Result<Option<Repo>> {
    let mut stmt = conn.prepare(
        "SELECT id,name,url,repo_type,enabled,priority,pubkey,last_sync,suite,components
         FROM repos WHERE name=?1",
    )?;
    let mut rows = stmt.query(params![name])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Repo {
            id:         row.get(0)?,
            name:       row.get(1)?,
            url:        row.get(2)?,
            repo_type:  row.get(3)?,
            enabled:    row.get::<_, i64>(4)? != 0,
            priority:   row.get(5)?,
            pubkey:     row.get(6)?,
            last_sync:  row.get(7)?,
            suite:      row.get(8)?,
            components: row.get(9)?,
        }))
    } else {
        Ok(None)
    }
}
