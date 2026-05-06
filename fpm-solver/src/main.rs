use anyhow::Result;
use clap::{Parser, Subcommand};
use fpm_solver::{
    index::{PackageIndex, PackageRecord},
    manifest::Manifest,
    solver::resolve,
    types::{Dep, Version, VersionReq},
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fpm-solver", about = "M1 Dependency Solver — resolve .fpkg dependencies")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Resolve {
        #[arg(long, help = "Root manifest.toml to resolve")]
        manifest: PathBuf,
        #[arg(long, help = "Directory containing package manifest.toml files")]
        index: PathBuf,
    },
    Check {
        #[arg(long, help = "manifest.toml to validate")]
        manifest: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Resolve { manifest, index } => cmd_resolve(&manifest, &index),
        Command::Check { manifest } => cmd_check(&manifest),
    }
}

fn cmd_check(manifest_path: &PathBuf) -> Result<()> {
    let src = std::fs::read_to_string(manifest_path)?;
    let m = Manifest::from_str(&src)?;
    println!("name:     {}", m.package.name);
    println!("version:  {}", m.package.version);
    let deps = m.required_deps();
    if deps.is_empty() {
        println!("deps:     none");
    } else {
        println!("deps:");
        for d in &deps {
            let opt = if d.optional { " (optional)" } else { "" };
            println!("  {} {}{}", d.name, d.req.version, opt);
        }
    }
    let provides = m.all_provides();
    if !provides.is_empty() {
        println!("provides: {}", provides.join(", "));
    }
    let conflicts = m.all_conflicts();
    if !conflicts.is_empty() {
        println!("conflicts: {}", conflicts.join(", "));
    }
    Ok(())
}

fn cmd_resolve(manifest_path: &PathBuf, index_dir: &PathBuf) -> Result<()> {
    let root_src = std::fs::read_to_string(manifest_path)?;
    let root_manifest = Manifest::from_str(&root_src)?;
    let root_name = root_manifest.package.name.clone();
    let root_version = Version::parse(&root_manifest.package.version)?;

    let mut idx = PackageIndex::new();

    for entry in std::fs::read_dir(index_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let src = std::fs::read_to_string(&path)?;
            if let Ok(m) = Manifest::from_str(&src) {
                let ver = Version::parse(&m.package.version)?;
                let deps = m.required_deps();
                let record = PackageRecord {
                    name: m.package.name.clone(),
                    version: ver,
                    deps,
                    provides: m.all_provides(),
                    conflicts: m.all_conflicts(),
                };
                idx.add(record);
            }
        }
    }

    let root_deps = root_manifest.required_deps();
    let root_record = PackageRecord {
        name: root_name.clone(),
        version: root_version.clone(),
        deps: root_deps,
        provides: root_manifest.all_provides(),
        conflicts: root_manifest.all_conflicts(),
    };
    idx.add(root_record);

    let resolution = resolve(&idx, &root_name, &root_version)?;

    println!("resolved {} package(s):", resolution.packages.len());
    let mut sorted: Vec<_> = resolution.iter().collect();
    sorted.sort_by_key(|(name, _)| name.as_str());
    for (name, version) in sorted {
        println!("  {} {}", name, version);
    }

    Ok(())
}
