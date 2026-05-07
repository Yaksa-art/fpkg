use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("script '{script}' exited with code {code}\nstdout: {stdout}\nstderr: {stderr}")]
    ScriptFailed {
        script: String,
        code: i32,
        stdout: String,
        stderr: String,
    },

    #[error("script '{script}' timed out after {secs}s")]
    Timeout { script: String, secs: u64 },

    #[error("bwrap exec failed: {0}")]
    BwrapExec(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}
