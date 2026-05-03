use anyhow::{bail, Result};
use rusqlite::{params, Connection};

use crate::models::{NewPackage, Package};

pub fn insert(conn: &Connection, pkg: &NewPackage) -> Result<i64> {
    conn.execute(
        "INSERT INTO packages
            (name, version, release, arch, mode, summary, license, maintainer,
             homepage, install_size, origin_format, install_date, manifest_hash, content_tree)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            pkg.name, pkg.version, pkg.release, pkg.arch,
            pkg.mode, pkg.summary, pkg.license, pkg.maintainer,
            pkg.homepage, pkg.install_size, pkg.origin_format,
            pkg.install_date, pkg.manifest_hash, pkg.content_tree,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, pkg: &NewPackage) -> Result<()> {
    let rows = conn.execute(
        "UPDATE packages SET
            version=?1, release=?2, arch=?3, summary=?4, license=?5,
            maintainer=?6, homepage=?7, install_size=?8, origin_format=?9,
            install_date=?10, manifest_hash=?11, content_tree=?12
         WHERE name=?13 AND mode=?14",
        params![
            pkg.version, pkg.release, pkg.arch, pkg.summary, pkg.license,
            pkg.maintainer, pkg.homepage, pkg.install_size, pkg.origin_format,
            pkg.install_date, pkg.manifest_hash, pkg.content_tree,
            pkg.name, pkg.mode,
        ],
    )?;
    if rows == 0 {
        bail!("Package not found: {} (mode={})", pkg.name, pkg.mode);
    }
    Ok(())
}

pub fn upsert(conn: &Connection, pkg: &NewPackage) -> Result<i64> {
    if get_by_name(conn, &pkg.name, &pkg.mode)?.is_some() {
        update(conn, pkg)?;
        Ok(get_by_name(conn, &pkg.name, &pkg.mode)?.unwrap().id)
    } else {
        insert(conn, pkg)
    }
}

pub fn remove(conn: &Connection, name: &str, mode: &str) -> Result<bool> {
    let rows = conn.execute(
        "DELETE FROM packages WHERE name=?1 AND mode=?2",
        params![name, mode],
    )?;
    Ok(rows > 0)
}

pub fn get_by_name(conn: &Connection, name: &str, mode: &str) -> Result<Option<Package>> {
    let mut stmt = conn.prepare(
        "SELECT id,name,version,release,arch,mode,summary,license,maintainer,
                homepage,install_size,origin_format,install_date,manifest_hash,content_tree
         FROM packages WHERE name=?1 AND mode=?2",
    )?;
    let mut rows = stmt.query(params![name, mode])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row_to_package(row)?))
    } else {
        Ok(None)
    }
}

pub fn list_all(conn: &Connection, mode: Option<&str>) -> Result<Vec<Package>> {
    let (sql, param): (&str, Option<&str>) = match mode {
        Some(m) => (
            "SELECT id,name,version,release,arch,mode,summary,license,maintainer,
                    homepage,install_size,origin_format,install_date,manifest_hash,content_tree
             FROM packages WHERE mode=?1 ORDER BY name",
            Some(m),
        ),
        None => (
            "SELECT id,name,version,release,arch,mode,summary,license,maintainer,
                    homepage,install_size,origin_format,install_date,manifest_hash,content_tree
             FROM packages ORDER BY name",
            None,
        ),
    };

    let mut stmt = conn.prepare(sql)?;
    let rows = if let Some(p) = param {
        stmt.query(params![p])?
    } else {
        stmt.query([])?
    };

    collect_packages(rows)
}

pub fn search(conn: &Connection, query: &str) -> Result<Vec<Package>> {
    let pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(
        "SELECT id,name,version,release,arch,mode,summary,license,maintainer,
                homepage,install_size,origin_format,install_date,manifest_hash,content_tree
         FROM packages WHERE name LIKE ?1 OR summary LIKE ?1 ORDER BY name",
    )?;
    let rows = stmt.query(params![pattern])?;
    collect_packages(rows)
}

pub fn is_held(conn: &Connection, name: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM hold WHERE name=?1",
        params![name],
        |r| r.get(0),
    )?;
    Ok(count > 0)
}

fn row_to_package(row: &rusqlite::Row) -> rusqlite::Result<Package> {
    Ok(Package {
        id:            row.get(0)?,
        name:          row.get(1)?,
        version:       row.get(2)?,
        release:       row.get(3)?,
        arch:          row.get(4)?,
        mode:          row.get(5)?,
        summary:       row.get(6)?,
        license:       row.get(7)?,
        maintainer:    row.get(8)?,
        homepage:      row.get(9)?,
        install_size:  row.get(10)?,
        origin_format: row.get(11)?,
        install_date:  row.get(12)?,
        manifest_hash: row.get(13)?,
        content_tree:  row.get(14)?,
    })
}

fn collect_packages(mut rows: rusqlite::Rows) -> Result<Vec<Package>> {
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(row_to_package(row)?);
    }
    Ok(out)
}
