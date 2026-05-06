pub mod index;
pub mod manifest;
pub mod solver;
pub mod types;

pub use index::PackageIndex;
pub use solver::{resolve, Resolution};
pub use types::{Dep, Package, VersionReq};
