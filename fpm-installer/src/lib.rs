//! fpm M5 Installer
//!
//! Responsibility: take a verified .fpkg archive + a Transaction from M4,
//! extract DATA/ into `trx.root_dir()`, run pre/post-install scripts,
//! write a per-package file manifest, then signal M4 to commit.
//!
//! Pipeline position:
//!   M1 (solve) → M2 (fetch) → M3 (verify) → M4 (trx.begin)
//!       → **M5 (install)** → M4 (trx.commit) → M8 (db record, upcoming)

pub mod error;
pub mod extract;
pub mod layout;
pub mod manifest;
pub mod hooks;
pub mod installer;
pub mod remove;

pub use error::InstallerError;
pub use installer::{Installer, InstallResult};
pub use remove::Remover;
