use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tracing::info;
use crate::{BuildError, PkgBuild};

pub struct BuildEnv {
    pub build_dir:  PathBuf,
    pub src_dir:    PathBuf,
    pub destdir:    PathBuf,
    pub script_path: PathBuf,
}

pub fn prepare(pb: &PkgBuild, workdir: &Path) -> Result<BuildEnv, BuildError> {
    let build_dir  = workdir.join("build");
    let src_dir    = workdir.join("src");
    let destdir    = workdir.join("pkg");
    let script_path = workdir.join("build.sh");

    for d in &[&build_dir, &src_dir, &destdir] {
        fs::create_dir_all(d)?;
    }

    fs::write(&script_path, &pb.build.script)?;
    fs::set_permissions(
        &script_path,
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    )?;

    if let Some(sources) = &pb.package.source {
        fetch_sources(sources, &src_dir)?;
    }

    info!(
        pkg = %pb.package.name,
        build_dir = %build_dir.display(),
        destdir   = %destdir.display(),
        "build environment prepared"
    );

    Ok(BuildEnv { build_dir, src_dir, destdir, script_path })
}

fn fetch_sources(
    sources: &[crate::pkgbuild::SourceEntry],
    src_dir: &Path,
) -> Result<(), BuildError> {
    for src in sources {
        let fname = src.url.split('/').last().unwrap_or("source");
        let dest  = src_dir.join(fname);
        if dest.exists() {
            continue;
        }
        info!(url = %src.url, "fetching source");
        let status = Command::new("curl")
            .args(["-fsSL", "-o", dest.to_str().unwrap(), &src.url])
            .status()?;
        if !status.success() {
            return Err(BuildError::Anyhow(
                anyhow::anyhow!("curl failed for {}", src.url)
            ));
        }
        if let Some(expected) = &src.sha256 {
            verify_sha256(&dest, expected)?;
        }
    }
    Ok(())
}

fn verify_sha256(path: &Path, expected: &str) -> Result<(), BuildError> {
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    let actual = hasher.finalize().to_hex().to_string();
    if !actual.starts_with(expected) {
        return Err(BuildError::Anyhow(
            anyhow::anyhow!("checksum mismatch: expected {}, got {}", expected, actual)
        ));
    }
    Ok(())
}

use std::os::unix::fs::PermissionsExt;
