use anyhow::Result;
use rusqlite::{params, Connection};

use crate::models::{NewFile, PackageFile};

pub fn insert_batch(conn: &Connection, package_id: i64, files: &[NewFile]) -> Result<()> {
    let mut stmt = conn.prepare(
        "INSERT OR REPLACE INTO files (package_id, path, hash, size, is_config)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for f in files {
        stmt.execute(params![
            package_id, f.path, f.hash, f.size, f.is_config as i64,
        ])?;
    }
    Ok(())
}

pub fn delete_for_package(conn: &Connection, package_id: i64) -> Result<usize> {
    let rows = conn.execute(
        "DELETE FROM files WHERE package_id=?1",
        params![package_id],
    )?;
    Ok(rows)
}

pub fn list_for_package(conn: &Connection, package_id: i64) -> Result<Vec<PackageFile>> {
    let mut stmt = conn.prepare(
        "SELECT id, package_id, path, hash, size, is_config FROM files
         WHERE package_id=?1 ORDER BY path",
    )?;
    let mut rows = stmt.query(params![package_id])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(PackageFile {
            id:         row.get(0)?,
            package_id: row.get(1)?,
            path:       row.get(2)?,
            hash:       row.get(3)?,
            size:       row.get(4)?,
            is_config:  row.get::<_, i64>(5)? != 0,
        });
    }
    Ok(out)
}

pub fn owner_of(conn: &Connection, path: &str) -> Result<Option<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT p.name, p.version, p.mode
         FROM files f JOIN packages p ON f.package_id = p.id
         WHERE f.path = ?1
         LIMIT 1",
    )?;
    let mut rows = stmt.query(params![path])?;
    if let Some(row) = rows.next()? {
        Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?)))
    } else {
        Ok(None)
    }
}

pub fn count_for_package(conn: &Connection, package_id: i64) -> Result<i64> {
    let count = conn.query_row(
        "SELECT COUNT(*) FROM files WHERE package_id=?1",
        params![package_id],
        |r| r.get(0),
    )?;
    Ok(count)
}
