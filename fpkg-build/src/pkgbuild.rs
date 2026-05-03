use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct PkgBuildPackage {
    pub name: String,
    pub version: String,
    #[serde(default = "default_release")]
    pub release: u32,
    #[serde(default = "default_arch")]
    pub arch: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub maintainer: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PkgBuildSource {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub local: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct PkgBuildBuild {
    #[serde(default)]
    pub build_depends: Vec<String>,
    #[serde(default)]
    pub script: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct PkgBuildInstall {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub script: String,
    #[serde(default)]
    pub config_files: Vec<String>,
    #[serde(default)]
    pub desktop_files: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PkgBuildRuntime {
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub suggests: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub provides: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PkgBuildScripts {
    #[serde(rename = "pre-install", default)]
    pub pre_install: String,
    #[serde(rename = "post-install", default)]
    pub post_install: String,
    #[serde(rename = "pre-remove", default)]
    pub pre_remove: String,
    #[serde(rename = "post-remove", default)]
    pub post_remove: String,
}

#[derive(Debug, Deserialize)]
pub struct PkgBuild {
    pub package: PkgBuildPackage,
    #[serde(default)]
    pub source: PkgBuildSource,
    #[serde(default)]
    pub build: PkgBuildBuild,
    #[serde(rename = "package_install", default)]
    pub install: PkgBuildInstall,
    #[serde(default)]
    pub runtime: PkgBuildRuntime,
    #[serde(default)]
    pub scripts: PkgBuildScripts,
    #[serde(default)]
    pub changelog: String,
}

impl PkgBuild {
    pub fn from_file(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)?;
        let pkgbuild: PkgBuild = toml::from_str(&raw)?;
        Ok(pkgbuild)
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.package.name.is_empty() {
            errors.push("package.name is required".into());
        }
        if self.package.version.is_empty() {
            errors.push("package.version is required".into());
        }
        let valid_arches = ["x86_64", "aarch64", "riscv64", "any"];
        if !valid_arches.contains(&self.package.arch.as_str()) {
            errors.push(format!(
                "package.arch must be one of: {}",
                valid_arches.join(" | ")
            ));
        }
        let valid_modes = ["system", "user", "both"];
        if !valid_modes.contains(&self.install.mode.as_str()) {
            errors.push("package_install.mode must be system | user | both".into());
        }
        errors
    }

    pub fn output_filename(&self) -> String {
        format!(
            "{}-{}-{}-{}.fpkg",
            self.package.name,
            self.package.version,
            self.package.release,
            self.package.arch
        )
    }
}

fn default_release() -> u32 { 1 }
fn default_arch() -> String { "x86_64".into() }
fn default_mode() -> String { "both".into() }
