use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A package row in the `packages` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbPackage {
    pub id: i64,
    pub name: String,
    pub version: String,
    /// Generation in which this package was installed
    pub generation_id: i64,
    /// BLAKE3 of the .fpkg archive
    pub blake3: Option<String>,
    /// Total installed size in bytes (sum of all files)
    pub installed_size: i64,
    pub installed_at: DateTime<Utc>,
    /// Manually installed (true) vs pulled in as dependency (false)
    pub explicit: bool,
}

/// A file row in the `files` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbFile {
    pub id: i64,
    pub package_id: i64,
    pub package_name: String,
    /// Relative to filesystem root (no leading /)
    pub path: String,
    pub blake3: String,
    pub size: i64,
    /// "file" | "symlink" | "directory"
    pub file_type: String,
}

/// A generation row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbGeneration {
    pub id: i64,
    pub gen_id: i64,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub parent_gen_id: Option<i64>,
    /// JSON snapshot of package list at this generation
    pub packages_json: String,
}

/// A hold row — packages locked against upgrade/remove.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbHold {
    pub id: i64,
    pub package_name: String,
    pub reason: Option<String>,
    pub held_at: DateTime<Utc>,
}
