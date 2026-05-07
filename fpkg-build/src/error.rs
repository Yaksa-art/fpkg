use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("PKGBUILD.toml parse error: {0}")]
    Parse(String),

    #[error("missing required field in PKGBUILD.toml: {0}")]
    MissingField(String),

    #[error("build script exited with status {0}")]
    BuildFailed(i32),

    #[error("DESTDIR is empty after build — nothing to package")]
    EmptyDestdir,

    #[error("packing .fpkg failed: {0}")]
    Pack(String),

    #[error("signing failed: {0}")]
    Sign(String),

    #[error("sandbox error: {0}")]
    Sandbox(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}
