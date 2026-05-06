//! fpm-core — M4 Transaction Manager + shared primitives.
//!
//! This crate is the backbone of the fpm daemon (fpmd).
//! It wires M1 (Solver) → M2 (Fetcher) → M4 (Transaction) → M5 (Installer, upcoming).

pub mod error;
pub mod generation;
pub mod plan;
pub mod trx;
pub mod paths;

pub use error::TrxError;
pub use generation::{Generation, GenerationId, GenerationMeta};
pub use plan::{InstallPlan, PlanEntry, PlanOp};
pub use trx::{Transaction, TransactionManager};
pub use paths::FpmPaths;

// Re-export upstream types so fpmd only needs fpm-core
pub use fpm_solver::{ResolvedPackage, Version};
pub use fpm_fetcher::FetchResult;
