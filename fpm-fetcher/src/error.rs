use thiserror::Error;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("HTTP error {status} for {url}")]
    Http { status: u16, url: String },

    #[error("All mirrors failed for package {package}")]
    AllMirrorsFailed { package: String },

    #[error("Checksum mismatch for {path}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        path: String,
        expected: String,
        actual: String,
    },

    #[error("M3 Verifier rejected package {package}: {reason}")]
    VerificationFailed { package: String, reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP client error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Package not found in any repo: {0}")]
    NotFound(String),
}
