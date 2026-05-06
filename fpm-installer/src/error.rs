use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstallerError {
    #[error("Package '{0}' has no .fpkg path — was it fetched?")]
    MissingFpkgPath(String),

    #[error(".fpkg archive is corrupt or not a tar.zst: {0}")]
    CorruptArchive(String),

    #[error("DATA/ entry has unsafe path (path traversal): {0}")]
    UnsafePath(String),

    #[error("File conflict: '{path}' already owned by package '{owner}'")]
    FileConflict { path: String, owner: String },

    #[error("Script '{script}' exited with code {code}")]
    ScriptFailed { script: String, code: i32 },

    #[error("Script timed out after {secs}s: {script}")]
    ScriptTimeout { script: String, secs: u64 },

    #[error("Manifest write failed for '{0}': {1}")]
    ManifestWrite(String, String),

    #[error("Remove error: file '{0}' not found in manifest")]
    NotInstalled(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Transaction error: {0}")]
    Trx(#[from] fpm_core::TrxError),
}
