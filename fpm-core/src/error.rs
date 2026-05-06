use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrxError {
    #[error("Generation {0} not found")]
    GenerationNotFound(u64),

    #[error("No current generation exists — database not initialised")]
    NoCurrentGeneration,

    #[error("Transaction already active (generation {0} pending)")]
    AlreadyActive(u64),

    #[error("Rollback target {target} is already current ({current})")]
    AlreadyCurrent { target: u64, current: u64 },

    #[error("Pending generation directory missing: {0}")]
    PendingDirMissing(String),

    #[error("Atomic rename failed: {0}")]
    AtomicRenameFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialisation error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Plan error: {0}")]
    Plan(String),
}
