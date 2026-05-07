use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use blake3::Hasher;
use walkdir::WalkDir;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;
use tracing::info;
use crate::{BuildError, PkgBuild};
use crate::prepare::BuildEnv;

pub struct PackResult {
    pub fpkg_path:  PathBuf,
    pub file_count: usize,
    pub blake3_manifest: String,
}

pub fn pack(pb: &PkgBuild, env: &BuildEnv, out_dir: &Path) -> Result<PackResult, BuildError> {
    let entries: Vec<PathBuf> = WalkDir::new(&env.destdir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect();

    if entries.is_empty() {
        return Err(BuildError::EmptyDestdir);
    }

    let fpkg_path = out_dir.join(pb.fpkg_name());
    let file = File::create(&fpkg_path)?;
    let mut zip = ZipWriter::new(file);
    let opts: FileOptions<()> = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let manifest_toml = build_manifest_toml(pb);
    zip.start_file("META/manifest.toml", opts)
        .map_err(|e| BuildError::Pack(e.to_string()))?;
    zip.write_all(manifest_toml.as_bytes())?;

    let mut checksums = String::new();
    let mut file_count = 0usize;

    for abs_path in &entries {
        let rel = abs_path
            .strip_prefix(&env.destdir)
            .map_err(|e| BuildError::Pack(e.to_string()))?;
        let arc_path = format!("DATA/{}", rel.display());

        let hash = hash_file(abs_path)?;
        checksums.push_str(&format!("{hash}  {arc_path}\n"));

        let metadata = fs::metadata(abs_path)?;
        let mode = std::os::unix::fs::MetadataExt::mode(&metadata);
        let file_opts: FileOptions<()> = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(mode);

        zip.start_file(&arc_path, file_opts)
            .map_err(|e| BuildError::Pack(e.to_string()))?;
        let mut f = File::open(abs_path)?;
        io::copy(&mut f, &mut zip)?;
        file_count += 1;
    }

    zip.start_file("META/checksums.blake3", opts)
        .map_err(|e| BuildError::Pack(e.to_string()))?;
    zip.write_all(checksums.as_bytes())?;

    zip.finish().map_err(|e| BuildError::Pack(e.to_string()))?;

    info!(
        pkg  = %pb.fpkg_name(),
        files = file_count,
        "packed .fpkg"
    );

    Ok(PackResult {
        fpkg_path,
        file_count,
        blake3_manifest: checksums,
    })
}

fn hash_file(path: &Path) -> Result<String, BuildError> {
    let mut f = File::open(path)?;
    let mut hasher = Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn build_manifest_toml(pb: &PkgBuild) -> String {
    let arches = pb.package.arch.join(", ");
    let deps = pb
        .runtime
        .as_ref()
        .and_then(|r| r.requires.as_ref())
        .map(|deps| {
            deps.iter()
                .map(|d| {
                    format!(
                        "[[dependencies]]\nname = \"{}\"\nversion = \"{}\"\noptional = {}\n",
                        d.name,
                        d.version.as_deref().unwrap_or("*"),
                        d.optional.unwrap_or(false),
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    format!(
        r#"[package]
name        = "{name}"
version     = "{version}"
release     = {release}
arch        = [{arch}]
license     = "{license}"
summary     = "{summary}"
homepage    = "{homepage}"
maintainer  = "{maintainer}"

[verification]
fpkg_format = 1

{deps}
"#,
        name       = pb.package.name,
        version    = pb.package.version,
        release    = pb.package.release,
        arch       = pb.package.arch.iter().map(|a| format!("\"{a}\"")).collect::<Vec<_>>().join(", "),
        license    = pb.package.license,
        summary    = pb.package.summary.as_deref().unwrap_or(""),
        homepage   = pb.package.homepage.as_deref().unwrap_or(""),
        maintainer = pb.package.maintainer.as_deref().unwrap_or(""),
        deps       = deps,
    )
}
