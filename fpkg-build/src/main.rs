mod builder;
mod checksums;
mod manifest;
mod package;
mod pkgbuild;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use builder::Builder;
use pkgbuild::PkgBuild;

#[derive(Parser)]
#[command(
    name = "fpkg-build",
    version = "0.1.1",
    about = "M10 Builder — build .fpkg packages from PKGBUILD.toml"
)]
struct Cli {
    #[arg(default_value = "PKGBUILD.toml", help = "Path to PKGBUILD.toml")]
    pkgbuild: PathBuf,

    #[arg(short, long, default_value = ".", help = "Output directory for the .fpkg")]
    output_dir: PathBuf,

    #[arg(short, long, help = "Show build script output")]
    verbose: bool,

    #[arg(long, help = "Validate PKGBUILD.toml without building")]
    dry_run: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if !cli.pkgbuild.exists() {
        eprintln!("[✗] Not found: {}", cli.pkgbuild.display());
        std::process::exit(1);
    }

    let pkg = PkgBuild::from_file(&cli.pkgbuild)?;

    let errors = pkg.validate();
    if !errors.is_empty() {
        eprintln!("[✗] Validation errors:");
        for e in &errors {
            eprintln!("    - {}", e);
        }
        std::process::exit(1);
    }

    if cli.dry_run {
        println!("[✓] {} is valid", cli.pkgbuild.display());
        println!("    Package : {} {}-{}", pkg.package.name, pkg.package.version, pkg.package.release);
        println!("    Arch    : {}", pkg.package.arch);
        println!("    Output  : {}", pkg.output_filename());
        println!(
            "    Requires: {}",
            if pkg.runtime.requires.is_empty() {
                "none".to_string()
            } else {
                pkg.runtime.requires.join(", ")
            }
        );
        return Ok(());
    }

    let builder = Builder::new(cli.output_dir, cli.verbose);
    builder.build(&pkg)?;

    Ok(())
}
