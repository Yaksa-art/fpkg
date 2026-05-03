use anyhow::Result;
use rusqlite::{params, Connection};

use crate::models::Hold;

pub fn add(conn: &Connection, name: &str, version: &str, reason: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO hold (name, version, reason) VALUES (?1, ?2, ?3)",
        params![name, version, reason],
    )?;
    Ok(())
}

pub fn remove(conn: &Connection, name: &str) -> Result<bool> {
    let rows = conn.execute("DELETE FROM hold WHERE name=?1", params![name])?;
    Ok(rows > 0)
}

pub fn list(conn: &Connection) -> Result<Vec<Hold>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, version, reason FROM hold ORDER BY name",
    )?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(Hold {
            id:      row.get(0)?,
            name:    row.get(1)?,
            version: row.get(2)?,
            reason:  row.get(3)?,
        });
    }
    Ok(out)
}

pub fn is_held(conn: &Connection, name: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM hold WHERE name=?1",
        params![name],
        |r| r.get(0),
    )?;
    Ok(count > 0)
}
