use crate::types::{Dep, VersionReq};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub package: PackageMeta,
    #[serde(default)]
    pub dependencies: Dependencies,
}

#[derive(Debug, Deserialize)]
pub struct PackageMeta {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Dependencies {
    #[serde(default)]
    pub requires: Vec<RawDep>,
    #[serde(default)]
    pub suggests: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub provides: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawDep {
    Simple(String),
    Full(RawDepFull),
}

#[derive(Debug, Deserialize)]
pub struct RawDepFull {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub optional: bool,
    pub reason: Option<String>,
}

impl Manifest {
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        let m: Manifest = toml::from_str(s)?;
        Ok(m)
    }

    pub fn required_deps(&self) -> Vec<Dep> {
        self.dependencies.requires.iter().map(|r| match r {
            RawDep::Simple(name) => Dep::required(name, VersionReq::any()),
            RawDep::Full(f) => Dep {
                name: f.name.clone(),
                req: if f.version.is_empty() { VersionReq::any() } else { VersionReq::parse(&f.version) },
                optional: f.optional,
                reason: f.reason.clone(),
            },
        }).collect()
    }

    pub fn all_provides(&self) -> Vec<String> {
        let mut out = self.package.provides.clone();
        out.extend(self.dependencies.provides.iter().cloned());
        out
    }

    pub fn all_conflicts(&self) -> Vec<String> {
        let mut out = self.package.conflicts.clone();
        out.extend(self.dependencies.conflicts.iter().cloned());
        out
    }
}
