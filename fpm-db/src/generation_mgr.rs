use rusqlite::params;
use serde_json::Value;

use crate::{db::Database, error::DbError, models::DbGeneration};

impl Database {
    pub fn snapshot_generation(&self, gen_id: i64) -> Result<String, DbError> {
        let pkgs = self.list_packages()?;
        let json = serde_json::to_string(&pkgs)
            .map_err(|e| DbError::Json(e.to_string()))?;
        self.conn.execute(
            "UPDATE generations SET packages_json = ?1 WHERE gen_id = ?2",
            params![json, gen_id],
        )?;
        Ok(json)
    }

    pub fn generation_snapshot(&self, gen_id: i64) -> Result<Option<Value>, DbError> {
        let gen = self.get_generation(gen_id)?;
        match gen {
            None => Ok(None),
            Some(g) => {
                let v: Value = serde_json::from_str(&g.packages_json)
                    .map_err(|e| DbError::Json(e.to_string()))?;
                Ok(Some(v))
            }
        }
    }

    pub fn prune_generations(&self, keep: usize) -> Result<usize, DbError> {
        let total: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM generations", [], |r| r.get(0)
        )?;

        if total <= keep as i64 {
            return Ok(0);
        }

        let to_delete = total as usize - keep;

        let mut stmt = self.conn.prepare(
            "SELECT gen_id FROM generations ORDER BY gen_id ASC LIMIT ?1"
        )?;
        let ids: Vec<i64> = stmt
            .query_map(params![to_delete as i64], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut pruned = 0;
        for id in &ids {
            self.conn.execute(
                "DELETE FROM packages WHERE generation_id = ?1",
                params![id],
            )?;
            self.conn.execute(
                "DELETE FROM generations WHERE gen_id = ?1",
                params![id],
            )?;
            pruned += 1;
        }

        Ok(pruned)
    }

    pub fn latest_generation(&self) -> Result<Option<DbGeneration>, DbError> {
        use rusqlite::OptionalExtension;
        use chrono::Utc;
        self.conn.query_row(
            "SELECT id, gen_id, description, created_at, parent_gen_id, packages_json
             FROM generations ORDER BY gen_id DESC LIMIT 1",
            [],
            |row| Ok(DbGeneration {
                id: row.get(0)?,
                gen_id: row.get(1)?,
                description: row.get(2)?,
                created_at: {
                    let s: String = row.get(3)?;
                    s.parse().unwrap_or(Utc::now())
                },
                parent_gen_id: row.get(4)?,
                packages_json: row.get(5)?,
            }),
        )
        .optional()
        .map_err(DbError::from)
    }

    pub fn generation_chain(&self, gen_id: i64) -> Result<Vec<DbGeneration>, DbError> {
        use chrono::Utc;
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE chain(id, gen_id, description, created_at, parent_gen_id, packages_json) AS (
                SELECT id, gen_id, description, created_at, parent_gen_id, packages_json
                FROM generations WHERE gen_id = ?1
                UNION ALL
                SELECT g.id, g.gen_id, g.description, g.created_at, g.parent_gen_id, g.packages_json
                FROM generations g
                JOIN chain c ON g.gen_id = c.parent_gen_id
             )
             SELECT id, gen_id, description, created_at, parent_gen_id, packages_json FROM chain"
        )?;
        let rows = stmt.query_map(params![gen_id], |row| {
            Ok(DbGeneration {
                id: row.get(0)?,
                gen_id: row.get(1)?,
                description: row.get(2)?,
                created_at: { let s: String = row.get(3)?; s.parse().unwrap_or(Utc::now()) },
                parent_gen_id: row.get(4)?,
                packages_json: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }
}
