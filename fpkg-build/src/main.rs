use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;
use fpkg_build::{pkgbuild::PkgBuild, BuildError};

#[derive(Debug, Parser)]
#[command(
    name  = "fpkg-build",
    about = "Build a .fpkg from PKGBUILD.toml"
)]
struct Cli {
    #[arg(help = "Path to PKGBUILD.toml")]
    pkgbuild: PathBuf,

    #[arg(long, help = "Sign the resulting .fpkg with fpkg-sign")]
    sign: bool,

    #[arg(long, help = "Path to signing key (passed to fpkg-sign)")]
    key: Option<PathBuf>,

    #[arg(long, help = "Output directory for .fpkg", default_value = ".")]
    outdir: PathBuf,

    #[arg(long, help = "Validate PKGBUILD.toml without building")]
    check: bool,

    #[arg(long, help = "Upload .fpkg after building (requires fpm-upload in PATH)")]
    upload: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let pb = PkgBuild::load(&cli.pkgbuild)
        .with_context(|| format!("loading {:?}", cli.pkgbuild))?;

    if cli.check {
        println!("PKGBUILD.toml is valid: {}-{}-{}",
            pb.package.name, pb.package.version, pb.package.release);
        return Ok(());
    }

    let workdir = tempfile::TempDir::new()
        .context("creating workdir")?;

    let env = fpkg_build::prepare::prepare(&pb, workdir.path())
        .context("preparing build environment")?;

    fpkg_build::runner::run_build(&pb, &env)
        .context("running build script")?;

    let result = fpkg_build::packer::pack(&pb, &env, &cli.outdir)
        .context("packing .fpkg")?;

    if cli.sign {
        fpkg_build::sign::sign_fpkg(&result.fpkg_path, cli.key.as_deref())
            .context("signing .fpkg")?;
    }

    if cli.upload {
        upload_fpkg(&result.fpkg_path)?;
    }

    println!("{}", result.fpkg_path.display());
    eprintln!("built {} ({} files)", result.fpkg_path.display(), result.file_count);

    Ok(())
}

fn upload_fpkg(fpkg_path: &PathBuf) -> Result<()> {
    let status = std::process::Command::new("fpm-upload")
        .arg(fpkg_path)
        .status()
        .context("fpm-upload not found")?;
    if !status.success() {
        anyhow::bail!("fpm-upload failed with status {:?}", status.code());
    }
    Ok(())
}
