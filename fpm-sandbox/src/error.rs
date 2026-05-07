use thiserror::Error;

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("unsupported sandbox level on this kernel: {0}")]
    UnsupportedLevel(String),

    #[error("overlay mount failed for '{pkg}': {source}")]
    OverlayMount {
        pkg: String,
        #[source]
        source: std::io::Error,
    },

    #[error("bwrap not found in PATH")]
    BwrapMissing,

    #[error("fuse-overlayfs not found in PATH")]
    FuseOverlayFsMissing,

    #[error("namespace operation failed: {0}")]
    Namespace(String),

    #[error("child process exited with status {0}")]
    ChildFailed(i32),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}
