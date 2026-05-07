use std::path::Path;
use tracing::debug;
use crate::{CompatError, convert::{ForeignDep, ForeignPackage}};

pub fn parse(path: &Path) -> Result<ForeignPackage, CompatError> {
    let mut f = std::fs::File::open(path)?;
    let pkg = rpm::Package::open(path)
        .map_err(|e| CompatError::Rpm(e.to_string()))?;

    let meta = pkg.metadata;

    let name    = meta.get_name().map_err(|e| CompatError::Rpm(e.to_string()))?.to_string();
    let version = meta.get_version().map_err(|e| CompatError::Rpm(e.to_string()))?.to_string();
    let release = meta.get_release().map_err(|e| CompatError::Rpm(e.to_string()))?.to_string();
    let arch    = meta.get_arch().map_err(|e| CompatError::Rpm(e.to_string()))?.to_string();
    let summary = meta.get_summary().map_err(|e| CompatError::Rpm(e.to_string()))?.to_string();
    let desc    = meta.get_description().map_err(|e| CompatError::Rpm(e.to_string()))?.to_string();
    let license = meta.get_license().map_err(|e| CompatError::Rpm(e.to_string()))?.to_string();
    let url     = meta.get_url().unwrap_or_default().to_string();

    let depends: Vec<ForeignDep> = meta
        .get_requires()
        .unwrap_or_default()
        .iter()
        .map(|d| ForeignDep {
            name:     d.name.clone(),
            version:  d.version.clone(),
            optional: false,
        })
        .collect();

    debug!(name = %name, "parsed .rpm header");

    Ok(ForeignPackage {
        format:         "rpm".into(),
        name,
        version,
        release,
        arch:           rpm_arch(&arch),
        summary,
        description:    desc,
        license,
        homepage:       url,
        maintainer:     String::new(),
        depends,
        conflicts:      vec![],
        provides:       vec![],
        installed_size: 0,
    })
}

fn rpm_arch(a: &str) -> String {
    match a {
        "x86_64"  => "x86_64",
        "aarch64" => "aarch64",
        "noarch"  => "any",
        other     => other,
    }.to_string()
}
