use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::BuildError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkgBuild {
    pub package:  PackageMeta,
    pub build:    BuildSection,
    pub runtime:  Option<RuntimeSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMeta {
    pub name:         String,
    pub version:      String,
    pub release:      u32,
    pub arch:         Vec<String>,
    pub license:      String,
    pub summary:      Option<String>,
    pub description:  Option<String>,
    pub homepage:     Option<String>,
    pub maintainer:   Option<String>,
    pub source:       Option<Vec<SourceEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEntry {
    pub url:    String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSection {
    pub build_depends: Option<Vec<String>>,
    pub script:        String,
    pub package_install: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSection {
    pub requires: Option<Vec<RuntimeDep>>,
    pub suggests: Option<Vec<String>>,
    pub conflicts: Option<Vec<String>>,
    pub provides:  Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildDep {
    pub name:    String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeDep {
    pub name:     String,
    pub version:  Option<String>,
    pub optional: Option<bool>,
}

impl PkgBuild {
    pub fn load(path: &Path) -> Result<Self, BuildError> {
        let raw = std::fs::read_to_string(path)?;
        let pb: PkgBuild = toml::from_str(&raw)
            .map_err(|e| BuildError::Parse(e.to_string()))?;
        pb.validate()?;
        Ok(pb)
    }

    fn validate(&self) -> Result<(), BuildError> {
        if self.package.name.is_empty() {
            return Err(BuildError::MissingField("package.name".into()));
        }
        if self.package.version.is_empty() {
            return Err(BuildError::MissingField("package.version".into()));
        }
        if self.package.arch.is_empty() {
            return Err(BuildError::MissingField("package.arch".into()));
        }
        if self.build.script.trim().is_empty() {
            return Err(BuildError::MissingField("build.script".into()));
        }
        Ok(())
    }

    pub fn fpkg_name(&self) -> String {
        format!(
            "{}-{}-{}.{}.fpkg",
            self.package.name,
            self.package.version,
            self.package.release,
            self.package.arch.first().map(String::as_str).unwrap_or("any"),
        )
    }
}
