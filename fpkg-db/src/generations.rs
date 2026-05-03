use anyhow::Result;
use rusqlite::{params, Connection};

use crate::models::{Generation, GenerationEntry};

pub fn record(conn: &Connection, action: &str, entries: &[GenerationEntry], note: &str) -> Result<i64> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let packages_json = serde_json::to_string(entries)?;
    conn.execute(
        "INSERT INTO generations (created_at, action, packages, note)
         VALUES (?1, ?2, ?3, ?4)",
        params![now, action, packages_json, note],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn mark_rolled_back(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE generations SET rolled_back=1 WHERE id=?1",
        params![id],
    )?;
    Ok(())
}

pub fn list(conn: &Connection, limit: usize) -> Result<Vec<Generation>> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, action, packages, note, rolled_back
         FROM generations ORDER BY id DESC LIMIT ?1",
    )?;
    let mut rows = stmt.query(params![limit as i64])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let packages_json: String = row.get(3)?;
        let packages: Vec<GenerationEntry> = serde_json::from_str(&packages_json)
            .unwrap_or_default();
        out.push(Generation {
            id:           row.get(0)?,
            created_at:   row.get(1)?,
            action:       row.get(2)?,
            packages,
            note:         row.get(4)?,
            rolled_back:  row.get::<_, i64>(5)? != 0,
        });
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Generation>> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, action, packages, note, rolled_back
         FROM generations WHERE id=?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        let packages_json: String = row.get(3)?;
        let packages: Vec<GenerationEntry> = serde_json::from_str(&packages_json)
            .unwrap_or_default();
        Ok(Some(Generation {
            id:          row.get(0)?,
            created_at:  row.get(1)?,
            action:      row.get(2)?,
            packages,
            note:        row.get(4)?,
            rolled_back: row.get::<_, i64>(5)? != 0,
        }))
    } else {
        Ok(None)
    }
}

pub fn latest_id(conn: &Connection) -> Result<Option<i64>> {
    let mut stmt = conn.prepare(
        "SELECT id FROM generations WHERE rolled_back=0 ORDER BY id DESC LIMIT 1",
    )?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

pub fn purge_old(conn: &Connection, keep: usize) -> Result<usize> {
    let rows = conn.execute(
        "DELETE FROM generations WHERE id NOT IN (
             SELECT id FROM generations ORDER BY id DESC LIMIT ?1
         )",
        params![keep as i64],
    )?;
    Ok(rows)
}
