use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ManifestSize {
    pub installed: u64,
    pub compressed: u64,
    pub delta_base: String,
}

#[derive(Debug, Serialize)]
pub struct ManifestFlags {
    pub system_config: bool,
    pub has_services: bool,
    pub selinux_aware: bool,
    pub has_suid: bool,
}

#[derive(Debug, Serialize)]
pub struct ManifestPackage {
    pub name: String,
    pub version: String,
    pub release: u32,
    pub arch: String,
    pub license: String,
    pub summary: String,
    pub description: String,
    pub homepage: String,
    pub source_url: String,
    pub maintainer: String,
    pub build_date: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub size: ManifestSize,
    pub flags: ManifestFlags,
}

#[derive(Debug, Serialize)]
pub struct ManifestVerification {
    pub manifest_hash: String,
    pub content_tree: String,
    pub signature_algo: String,
}

#[derive(Debug, Serialize)]
pub struct ManifestDependencies {
    pub requires: Vec<String>,
    pub suggests: Vec<String>,
    pub conflicts: Vec<String>,
    pub provides: Vec<String>,
    pub before: Vec<String>,
    pub after: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ManifestInstall {
    pub mode: String,
    pub config_files: Vec<String>,
    pub desktop_files: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ManifestRepository {
    pub origin_repo: String,
    pub origin_url: String,
    pub fetched_at: String,
}

#[derive(Debug, Serialize)]
pub struct ManifestCompat {
    pub min_fpm_version: String,
    pub fpkg_format: String,
}

#[derive(Debug, Serialize)]
pub struct Manifest {
    pub package: ManifestPackage,
    pub verification: ManifestVerification,
    pub dependencies: ManifestDependencies,
    pub install: ManifestInstall,
    pub repository: ManifestRepository,
    pub compat: ManifestCompat,
}

impl Manifest {
    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn build_date() -> String {
        Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}
