use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("package '{0}' not found")]
    PackageNotFound(String),

    #[error("generation {0} not found")]
    GenerationNotFound(u64),

    #[error("package '{0}' is held")]
    PackageHeld(String),

    #[error("json error: {0}")]
    Json(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("pool error: {0}")]
    Pool(String),

    #[error("serde_json: {0}")]
    SerdeJson(#[from] serde_json::Error),
}
