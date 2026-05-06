use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Package '{0}' not found in database")]
    PackageNotFound(String),

    #[error("Generation {0} not found in database")]
    GenerationNotFound(u64),

    #[error("Package '{0}' is held and cannot be modified")]
    PackageHeld(String),

    #[error("JSON serialisation error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
