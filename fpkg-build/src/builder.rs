use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use walkdir::WalkDir;

use crate::manifest::{
    Manifest, ManifestCompat, ManifestDependencies, ManifestFlags, ManifestInstall,
    ManifestPackage, ManifestRepository, ManifestSize, ManifestVerification,
};
use crate::package::FpkgWriter;
use crate::pkgbuild::PkgBuild;

pub struct Builder {
    pub output_dir: PathBuf,
    pub verbose: bool,
}

impl Builder {
    pub fn new(output_dir: PathBuf, verbose: bool) -> Self {
        Self { output_dir, verbose }
    }

    pub fn build(&self, pkg: &PkgBuild) -> Result<PathBuf> {
        let output_path = self.output_dir.join(pkg.output_filename());

        println!(
            "[*] Building {} {}-{} ({})",
            pkg.package.name, pkg.package.version, pkg.package.release, pkg.package.arch
        );

        let workdir = TempDir::new().context("Cannot create temp directory")?;
        let srcdir = workdir.path().join("src");
        let destdir = workdir.path().join("dest");
        fs::create_dir_all(&srcdir)?;
        fs::create_dir_all(&destdir)?;

        self.prepare_source(pkg, &srcdir)?;
        self.run_script_phase("build", &pkg.build.script, &srcdir, &destdir, pkg)?;
        self.run_script_phase("install", &pkg.install.script, &srcdir, &destdir, pkg)?;

        let manifest = Self::build_manifest(pkg);
        let mut writer = FpkgWriter::new(output_path.to_string_lossy().to_string(), manifest);

        let mut file_count = 0usize;
        for entry in WalkDir::new(&destdir).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry.path().strip_prefix(&destdir)?;
            let content = fs::read(entry.path())?;
            writer.add_data_file(&rel.to_string_lossy(), content);
            file_count += 1;
        }

        Self::attach_scripts(&mut writer, pkg);

        if !pkg.changelog.is_empty() {
            writer.changelog = pkg.changelog.clone();
        }

        println!("[*] Packing {} file(s)...", file_count);
        let compressed = writer.write()?;
        println!(
            "[✓] Package created: {}",
            output_path.display()
        );
        println!("    Compressed: {} bytes", compressed);

        Ok(output_path)
    }

    fn prepare_source(&self, pkg: &PkgBuild, srcdir: &Path) -> Result<()> {
        if !pkg.source.local.is_empty() {
            let local = PathBuf::from(&pkg.source.local);
            if !local.exists() {
                bail!("Local source not found: {}", pkg.source.local);
            }
            if local.is_dir() {
                copy_dir(&local, srcdir)?;
            } else {
                fs::copy(&local, srcdir.join(local.file_name().unwrap()))?;
            }
            println!("[✓] Source: {}", pkg.source.local);
        } else if !pkg.source.url.is_empty() {
            println!("[*] Fetching: {}", pkg.source.url);
            self.fetch_url(&pkg.source.url, &pkg.source.sha256, srcdir)?;
            println!("[✓] Source fetched");
        } else {
            println!("[!] No source — using empty DATA/");
        }
        Ok(())
    }

    fn fetch_url(&self, url: &str, expected_sha256: &str, destdir: &Path) -> Result<()> {
        let filename = url.split('/').last().unwrap_or("source");
        let dest_file = destdir.join(filename);

        let status = Command::new("curl")
            .args(["-fsSL", "-o", &dest_file.to_string_lossy(), url])
            .status()
            .context("curl not found")?;

        if !status.success() {
            bail!("Download failed: {}", url);
        }

        if !expected_sha256.is_empty() {
            let data = fs::read(&dest_file)?;
            let actual = format!("blake3:{}", blake3::hash(&data).to_hex());
            let expected_normalized = if expected_sha256.starts_with("blake3:") {
                expected_sha256.to_string()
            } else {
                format!("sha256:{}", expected_sha256)
            };
            if actual != expected_normalized && format!("sha256:{}", expected_sha256) != expected_normalized {
                bail!("Checksum mismatch for {}", filename);
            }
        }

        let name = filename.to_lowercase();
        if name.ends_with(".tar.gz") || name.ends_with(".tgz")
            || name.ends_with(".tar.bz2") || name.ends_with(".tar.xz")
            || name.ends_with(".tar.zst")
        {
            let status = Command::new("tar")
                .args(["xf", &dest_file.to_string_lossy(), "-C", &destdir.to_string_lossy()])
                .status()
                .context("tar not found")?;
            if !status.success() {
                bail!("Failed to extract archive");
            }
            fs::remove_file(&dest_file)?;
        }

        Ok(())
    }

    fn run_script_phase(
        &self,
        phase: &str,
        script: &str,
        srcdir: &Path,
        destdir: &Path,
        pkg: &PkgBuild,
    ) -> Result<()> {
        if script.trim().is_empty() {
            return Ok(());
        }

        println!("[*] Running {} script...", phase);

        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-e")
            .arg("-c")
            .arg(script)
            .env("FPM_SRCDIR", srcdir)
            .env("FPM_DESTDIR", destdir)
            .env("FPM_NAME", &pkg.package.name)
            .env("FPM_VERSION", &pkg.package.version)
            .env("FPM_ARCH", &pkg.package.arch)
            .env("FPM_MODE", "build");

        if self.verbose {
            let status = cmd.status().context("Failed to run script")?;
            if !status.success() {
                bail!("Script failed (exit {})", status.code().unwrap_or(-1));
            }
        } else {
            let output = cmd.output().context("Failed to run script")?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!(
                    "{} script failed (exit {}):\n{}",
                    phase,
                    output.status.code().unwrap_or(-1),
                    stderr
                );
            }
        }

        println!("[✓] {} complete", phase);
        Ok(())
    }

    fn attach_scripts(writer: &mut FpkgWriter, pkg: &PkgBuild) {
        let pairs = [
            ("pre-install.sh", &pkg.scripts.pre_install),
            ("post-install.sh", &pkg.scripts.post_install),
            ("pre-remove.sh", &pkg.scripts.pre_remove),
            ("post-remove.sh", &pkg.scripts.post_remove),
        ];
        for (name, content) in &pairs {
            if !content.is_empty() {
                writer.add_script(name, content.as_bytes().to_vec());
            }
        }
    }

    fn build_manifest(pkg: &PkgBuild) -> Manifest {
        let source_url = if !pkg.source.url.is_empty() {
            pkg.source.url.clone()
        } else {
            pkg.source.local.clone()
        };

        Manifest {
            package: ManifestPackage {
                name: pkg.package.name.clone(),
                version: pkg.package.version.clone(),
                release: pkg.package.release,
                arch: pkg.package.arch.clone(),
                license: pkg.package.license.clone(),
                summary: pkg.package.summary.clone(),
                description: pkg.package.description.clone(),
                homepage: pkg.package.homepage.clone(),
                source_url,
                maintainer: pkg.package.maintainer.clone(),
                build_date: Manifest::build_date(),
                categories: pkg.package.categories.clone(),
                tags: pkg.package.tags.clone(),
                size: ManifestSize { installed: 0, compressed: 0, delta_base: String::new() },
                flags: ManifestFlags {
                    system_config: false,
                    has_services: false,
                    selinux_aware: false,
                    has_suid: false,
                },
            },
            verification: ManifestVerification {
                manifest_hash: String::new(),
                content_tree: String::new(),
                signature_algo: "ed25519".into(),
            },
            dependencies: ManifestDependencies {
                requires: pkg.runtime.requires.clone(),
                suggests: pkg.runtime.suggests.clone(),
                conflicts: pkg.runtime.conflicts.clone(),
                provides: pkg.runtime.provides.clone(),
                before: Vec::new(),
                after: Vec::new(),
            },
            install: ManifestInstall {
                mode: pkg.install.mode.clone(),
                config_files: pkg.install.config_files.clone(),
                desktop_files: pkg.install.desktop_files.clone(),
            },
            repository: ManifestRepository {
                origin_repo: String::new(),
                origin_url: String::new(),
                fetched_at: String::new(),
            },
            compat: ManifestCompat {
                min_fpm_version: "0.1.0".into(),
                fpkg_format: "1".into(),
            },
        }
    }
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let rel = entry.path().strip_prefix(src)?;
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
