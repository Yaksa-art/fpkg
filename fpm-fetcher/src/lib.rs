pub mod cache;
pub mod config;
pub mod download;
pub mod error;
pub mod mirror;
pub mod progress;
pub mod verifier_ffi;

pub use config::FetcherConfig;
pub use download::{fetch_packages, FetchResult};
pub use error::FetchError;
pub use mirror::Mirror;
pub use progress::{ProgressEvent, ProgressSender};

// Re-export solver types so daemon/CLI can use one crate
pub use fpm_solver::{ResolvedPackage, Version};
