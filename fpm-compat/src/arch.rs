use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};
use tracing::debug;
use crate::{CompatError, convert::{ForeignDep, ForeignPackage}};

pub fn parse(path: &Path) -> Result<ForeignPackage, CompatError> {
    let f = File::open(path)?;
    let tar_data: Box<dyn Read> = if path.to_str().map_or(false, |p| p.ends_with(".zst")) {
        Box::new(zstd::stream::read::Decoder::new(f)
            .map_err(|e| CompatError::Archive(e.to_string()))?)
    } else if path.to_str().map_or(false, |p| p.ends_with(".xz")) {
        return Err(CompatError::Archive(".xz decompression not built-in; use zstd variant".into()));
    } else {
        Box::new(flate2::read::GzDecoder::new(f))
    };

    let mut tar = tar::Archive::new(tar_data);

    for entry in tar.entries().map_err(|e| CompatError::Archive(e.to_string()))? {
        let mut entry = entry.map_err(|e| CompatError::Archive(e.to_string()))?;
        let name = {
            let p = entry.path().map_err(|e| CompatError::Archive(e.to_string()))?;
            p.to_str().unwrap_or("").to_string()
        };
        if name == ".PKGINFO" {
            let mut buf = String::new();
            entry.read_to_string(&mut buf)?;
            return parse_pkginfo(&buf);
        }
    }
    Err(CompatError::Archive(".PKGINFO not found in .pkg.tar.zst".into()))
}

fn parse_pkginfo(s: &str) -> Result<ForeignPackage, CompatError> {
    let mut name        = String::new();
    let mut version     = String::new();
    let mut arch        = String::new();
    let mut desc        = String::new();
    let mut url         = String::new();
    let mut license     = String::new();
    let mut size: u64   = 0;
    let mut depends     = Vec::new();
    let mut conflicts   = Vec::new();
    let mut provides    = Vec::new();

    for line in s.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() { continue; }
        if let Some((k, v)) = line.split_once(" = ") {
            match k.trim() {
                "pkgname"  => name     = v.trim().to_string(),
                "pkgver"   => version  = v.trim().to_string(),
                "arch"     => arch     = v.trim().to_string(),
                "pkgdesc"  => desc     = v.trim().to_string(),
                "url"      => url      = v.trim().to_string(),
                "license"  => license  = v.trim().to_string(),
                "size"     => size     = v.trim().parse().unwrap_or(0),
                "depend"   => depends.push(ForeignDep {
                    name:     v.trim().to_string(),
                    version:  None,
                    optional: false,
                }),
                "conflict" => conflicts.push(v.trim().to_string()),
                "provides" => provides.push(v.trim().to_string()),
                _ => {}
            }
        }
    }

    if name.is_empty() { return Err(CompatError::MissingField("pkgname".into())); }
    if version.is_empty() { return Err(CompatError::MissingField("pkgver".into())); }

    debug!(name = %name, "parsed Arch .PKGINFO");

    Ok(ForeignPackage {
        format:         "arch".into(),
        name,
        version,
        release:        "1".into(),
        arch:           arch_arch(&arch),
        summary:        desc.clone(),
        description:    desc,
        license,
        homepage:       url,
        maintainer:     String::new(),
        depends,
        conflicts,
        provides,
        installed_size: size,
    })
}

fn arch_arch(a: &str) -> String {
    match a {
        "x86_64"  => "x86_64",
        "aarch64" => "aarch64",
        "any"     => "any",
        other     => other,
    }.to_string()
}
