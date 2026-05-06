//! Higher-level query helpers built on top of `Database`.

use crate::{db::Database, error::DbError, models::DbPackage};

pub trait QueryExt {
    /// Search installed packages by name substring.
    fn search(&self, query: &str) -> Result<Vec<DbPackage>, DbError>;

    /// Packages installed in a specific generation.
    fn packages_in_generation(&self, gen_id: i64) -> Result<Vec<DbPackage>, DbError>;

    /// Return all packages that are NOT held.
    fn upgradeable(&self) -> Result<Vec<DbPackage>, DbError>;

    /// Return packages installed as explicit (not as dependency).
    fn explicit_packages(&self) -> Result<Vec<DbPackage>, DbError>;
}

impl QueryExt for Database {
    fn search(&self, query: &str) -> Result<Vec<DbPackage>, DbError> {
        use rusqlite::params;
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, name, version, generation_id, blake3, installed_size, installed_at, explicit \
             FROM packages WHERE name LIKE ?1 ORDER BY name"
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            use chrono::Utc;
            Ok(DbPackage {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                generation_id: row.get(3)?,
                blake3: row.get(4)?,
                installed_size: row.get(5)?,
                installed_at: { let s: String = row.get(6)?; s.parse().unwrap_or(Utc::now()) },
                explicit: row.get::<_, i64>(7)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn packages_in_generation(&self, gen_id: i64) -> Result<Vec<DbPackage>, DbError> {
        use rusqlite::params;
        let mut stmt = self.conn.prepare(
            "SELECT id, name, version, generation_id, blake3, installed_size, installed_at, explicit \
             FROM packages WHERE generation_id = ?1 ORDER BY name"
        )?;
        let rows = stmt.query_map(params![gen_id], |row| {
            use chrono::Utc;
            Ok(DbPackage {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                generation_id: row.get(3)?,
                blake3: row.get(4)?,
                installed_size: row.get(5)?,
                installed_at: { let s: String = row.get(6)?; s.parse().unwrap_or(Utc::now()) },
                explicit: row.get::<_, i64>(7)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn upgradeable(&self) -> Result<Vec<DbPackage>, DbError> {
        use rusqlite::params;
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.name, p.version, p.generation_id, p.blake3, \
                    p.installed_size, p.installed_at, p.explicit \
             FROM packages p \
             WHERE p.name NOT IN (SELECT package_name FROM holds) \
             ORDER BY p.name"
        )?;
        let rows = stmt.query_map([], |row| {
            use chrono::Utc;
            Ok(DbPackage {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                generation_id: row.get(3)?,
                blake3: row.get(4)?,
                installed_size: row.get(5)?,
                installed_at: { let s: String = row.get(6)?; s.parse().unwrap_or(Utc::now()) },
                explicit: row.get::<_, i64>(7)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn explicit_packages(&self) -> Result<Vec<DbPackage>, DbError> {
        use rusqlite::params;
        let mut stmt = self.conn.prepare(
            "SELECT id, name, version, generation_id, blake3, installed_size, installed_at, explicit \
             FROM packages WHERE explicit = 1 ORDER BY name"
        )?;
        let rows = stmt.query_map([], |row| {
            use chrono::Utc;
            Ok(DbPackage {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                generation_id: row.get(3)?,
                blake3: row.get(4)?,
                installed_size: row.get(5)?,
                installed_at: { let s: String = row.get(6)?; s.parse().unwrap_or(Utc::now()) },
                explicit: row.get::<_, i64>(7)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }
}

// Give query.rs access to the private conn field via a pub(crate) extension.
// We use a helper trait approach above; conn must be pub(crate) in db.rs.
// (See db.rs — conn is pub(crate).)
