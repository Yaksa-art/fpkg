//! fpm M8 Database
//!
//! Single SQLite file at `FpmPaths::db_path()` (default: /var/lib/fpm/db.sqlite).
//!
//! Schema:
//!   packages    — one row per installed package (name, version, gen_id, …)
//!   files       — one row per installed file (path, blake3, size, pkg_id)
//!   generations — mirrors GenerationMeta from M4
//!   holds       — packages locked against upgrade/remove

pub mod db;
pub mod error;
pub mod models;
pub mod query;
pub mod sync;

pub use db::Database;
pub use error::DbError;
pub use models::{DbFile, DbGeneration, DbHold, DbPackage};
pub use query::QueryExt;
pub use sync::DbSync;
