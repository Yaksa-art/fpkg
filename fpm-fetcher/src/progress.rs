use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Events emitted during download; consumed by CLI/TUI/daemon for progress display.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProgressEvent {
    /// Download started for a package
    Started {
        package: String,
        version: String,
        total_bytes: u64,
    },
    /// Bytes received since last Chunk event
    Chunk {
        package: String,
        received_bytes: u64,
        total_bytes: u64,
    },
    /// Download finished, verification starting
    Downloaded { package: String },
    /// M3 Verifier result
    Verified { package: String, ok: bool, reason: Option<String> },
    /// Fully done (cached or freshly downloaded + verified)
    Done { package: String, path: String },
    /// Error
    Error { package: String, reason: String },
}

pub type ProgressSender = mpsc::Sender<ProgressEvent>;
pub type ProgressReceiver = mpsc::Receiver<ProgressEvent>;

pub fn progress_channel(buffer: usize) -> (ProgressSender, ProgressReceiver) {
    mpsc::channel(buffer)
}
