use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompatError {
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("malformed control field '{field}': {reason}")]
    MalformedField { field: String, reason: String },

    #[error("missing required field: {0}")]
    MissingField(String),

    #[error("ar/tar extraction failed: {0}")]
    Archive(String),

    #[error("rpm parse error: {0}")]
    Rpm(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}
