//! fpm-installer CLI
//!
//! Usage:
//!   fpm-installer install --fpkg <path> --name <name> --version <ver> --root <staging-root>
//!   fpm-installer remove  --name <name> --version <ver> --root <real-root>
//!   fpm-installer list    --root <root>
//!   fpm-installer manifest --name <name> --version <ver> --root <root>

use std::path::PathBuf;
use fpm_installer::{
    installer::Installer,
    remove::Remover,
    manifest::PackageManifest,
    extract::extract_data,
};
use fpm_core::plan::{InstallPlan, PlanEntry, PlanOp};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: fpm-installer <install|remove|list|manifest> [options]");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "extract" => {
            // Low-level: extract one .fpkg DATA/ into a directory
            // fpm-installer extract <fpkg> <dest>
            if args.len() < 4 {
                eprintln!("Usage: fpm-installer extract <fpkg> <dest>");
                std::process::exit(1);
            }
            let fpkg = PathBuf::from(&args[2]);
            let dest = PathBuf::from(&args[3]);
            std::fs::create_dir_all(&dest)?;
            let files = extract_data(&fpkg, &dest)?;
            for f in &files {
                println!("{:<64}  {}", f.blake3, f.rel_path.display());
            }
            println!("\n{} files extracted to {}", files.len(), dest.display());
        }

        "remove" => {
            // fpm-installer remove --name <name> --version <ver> [--root <root>]
            let name    = flag(&args, "--name").unwrap_or_default();
            let version = flag(&args, "--version").unwrap_or_default();
            let root    = flag(&args, "--root").unwrap_or_else(|| "/".into());
            let remover = Remover::new(root);
            let deleted = remover.remove(&name, &version)?;
            println!("Removed {} {} ({} files deleted)", name, version, deleted);
        }

        "list" => {
            // fpm-installer list [--root <root>]
            let root = flag(&args, "--root").unwrap_or_else(|| "/".into());
            let installed = PackageManifest::list_installed(std::path::Path::new(&root));
            if installed.is_empty() {
                println!("No packages installed.");
            } else {
                for (name, ver) in &installed {
                    println!("{} {}", name, ver);
                }
            }
        }

        "manifest" => {
            // fpm-installer manifest --name <name> --version <ver> [--root <root>]
            let name    = flag(&args, "--name").unwrap_or_default();
            let version = flag(&args, "--version").unwrap_or_default();
            let root    = flag(&args, "--root").unwrap_or_else(|| "/".into());
            let m = PackageManifest::load(std::path::Path::new(&root), &name, &version)?;
            println!("Package: {} {}", m.name, m.version);
            println!("Files: {}", m.files.len());
            for f in &m.files {
                println!("  {:<64}  {}", f.blake3, f.path);
            }
        }

        other => {
            eprintln!("Unknown command: {}", other);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].clone())
}
