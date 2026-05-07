use std::path::Path;
use crate::{apk, arch, deb, rpm, CompatError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForeignFormat {
    Deb,
    Rpm,
    Apk,
    Arch,
}

impl ForeignFormat {
    pub fn detect(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?;
        if name.ends_with(".deb")                          { return Some(Self::Deb); }
        if name.ends_with(".rpm")                          { return Some(Self::Rpm); }
        if name.ends_with(".apk")                          { return Some(Self::Apk); }
        if name.ends_with(".pkg.tar.zst")
        || name.ends_with(".pkg.tar.xz")
        || name.ends_with(".pkg.tar.gz")                   { return Some(Self::Arch); }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignDep {
    pub name:     String,
    pub version:  Option<String>,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignPackage {
    pub format:       String,
    pub name:         String,
    pub version:      String,
    pub release:      String,
    pub arch:         String,
    pub summary:      String,
    pub description:  String,
    pub license:      String,
    pub homepage:     String,
    pub maintainer:   String,
    pub depends:      Vec<ForeignDep>,
    pub conflicts:    Vec<String>,
    pub provides:     Vec<String>,
    pub installed_size: u64,
}

pub fn convert(path: &Path) -> Result<ForeignPackage, CompatError> {
    let fmt = ForeignFormat::detect(path)
        .ok_or_else(|| CompatError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string(),
        ))?;

    match fmt {
        ForeignFormat::Deb  => deb::parse(path),
        ForeignFormat::Rpm  => rpm::parse(path),
        ForeignFormat::Apk  => apk::parse(path),
        ForeignFormat::Arch => arch::parse(path),
    }
}

pub fn to_manifest_toml(pkg: &ForeignPackage) -> String {
    let deps_toml = pkg
        .depends
        .iter()
        .map(|d| {
            let ver = d.version.as_deref().unwrap_or("*");
            format!(
                "[[dependencies]]\nname = \"{}\"\nversion = \"{}\"\noptional = {}\n",
                d.name, ver, d.optional
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"[package]
name        = "{name}"
version     = "{version}"
release     = "{release}"
arch        = "{arch}"
license     = "{license}"
summary     = "{summary}"
description = """
{description}
"""
homepage    = "{homepage}"
maintainer  = "{maintainer}"

[compat]
origin_format = "{format}"

{deps_toml}
"#,
        name        = pkg.name,
        version     = pkg.version,
        release     = pkg.release,
        arch        = pkg.arch,
        license     = pkg.license,
        summary     = pkg.summary,
        description = pkg.description,
        homepage    = pkg.homepage,
        maintainer  = pkg.maintainer,
        format      = pkg.format,
        deps_toml   = deps_toml,
    )
}
