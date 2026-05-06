//! fpm-db CLI
//!
//! Usage:
//!   fpm-db list                           # list installed packages
//!   fpm-db info    <name>                 # package details
//!   fpm-db files   <name>                 # files owned by package
//!   fpm-db owns    <path>                 # which package owns a file
//!   fpm-db gens                           # list generation history
//!   fpm-db hold    <name> [reason]        # hold package
//!   fpm-db unhold  <name>                 # remove hold
//!   fpm-db holds                          # list held packages
//!   fpm-db search  <query>               # search installed by name
//!   fpm-db stats                          # total packages + size
//!   fpm-db resync  [--lib <path>]         # full resync from disk

use fpm_core::paths::FpmPaths;
use fpm_db::{Database, DbSync, QueryExt};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: fpm-db <command> [args...]");
        std::process::exit(1);
    }

    let paths = if let Some(lib) = flag(&args, "--lib") {
        FpmPaths {
            lib_dir:   PathBuf::from(&lib),
            cache_dir: PathBuf::from("/var/cache/fpm"),
            log_dir:   PathBuf::from("/var/log"),
        }
    } else {
        FpmPaths::system()
    };

    let db = Database::open(&paths.db_path())?;

    match args[1].as_str() {
        "list" => {
            let pkgs = db.list_packages()?;
            if pkgs.is_empty() {
                println!("No packages installed.");
            } else {
                println!("{:<30} {:<15} {}", "NAME", "VERSION", "SIZE");
                println!("{}", "-".repeat(60));
                for p in &pkgs {
                    println!("{:<30} {:<15} {} bytes",
                        p.name, p.version, p.installed_size);
                }
                println!("\nTotal: {} package(s)", pkgs.len());
            }
        }

        "info" => {
            let name = args.get(2).cloned().unwrap_or_default();
            match db.get_package(&name)? {
                None => println!("Package '{}' not installed.", name),
                Some(p) => {
                    println!("Name:      {}", p.name);
                    println!("Version:   {}", p.version);
                    println!("Installed: {}", p.installed_at);
                    println!("Size:      {} bytes", p.installed_size);
                    println!("Explicit:  {}", p.explicit);
                    println!("Held:      {}", db.is_held(&p.name)?);
                    if let Some(b3) = &p.blake3 {
                        println!("BLAKE3:    {}", b3);
                    }
                }
            }
        }

        "files" => {
            let name = args.get(2).cloned().unwrap_or_default();
            let files = db.files_of(&name)?;
            if files.is_empty() {
                println!("No files recorded for '{}'.", name);
            } else {
                for f in &files {
                    println!("{}", f.path);
                }
                println!("\n{} file(s)", files.len());
            }
        }

        "owns" => {
            let path = args.get(2).cloned().unwrap_or_default();
            match db.owner_of(&path)? {
                None => println!("'{}' is not owned by any installed package.", path),
                Some(owner) => println!("{}: owned by {}", path, owner),
            }
        }

        "gens" => {
            let gens = db.list_generations()?;
            if gens.is_empty() {
                println!("No generations recorded.");
            } else {
                println!("{:<6} {:<30} {}", "GEN", "DATE", "DESCRIPTION");
                println!("{}", "-".repeat(70));
                for g in &gens {
                    println!("{:<6} {:<30} {}",
                        g.gen_id,
                        g.created_at.format("%Y-%m-%d %H:%M:%S"),
                        g.description);
                }
            }
        }

        "hold" => {
            let name   = args.get(2).cloned().unwrap_or_default();
            let reason = args.get(3).map(|s| s.as_str());
            db.hold(&name, reason)?;
            println!("Held: {}", name);
        }

        "unhold" => {
            let name = args.get(2).cloned().unwrap_or_default();
            if db.unhold(&name)? {
                println!("Unheld: {}", name);
            } else {
                println!("'{}' was not held.", name);
            }
        }

        "holds" => {
            let holds = db.list_holds()?;
            if holds.is_empty() {
                println!("No held packages.");
            } else {
                for h in &holds {
                    let reason = h.reason.as_deref().unwrap_or("no reason");
                    println!("{} ({}) — held since {}", h.package_name, reason, h.held_at);
                }
            }
        }

        "search" => {
            let query = args.get(2).cloned().unwrap_or_default();
            let pkgs = db.search(&query)?;
            if pkgs.is_empty() {
                println!("No matches for '{}'.", query);
            } else {
                for p in &pkgs {
                    println!("{} {}", p.name, p.version);
                }
            }
        }

        "stats" => {
            let count = db.package_count()?;
            let size  = db.total_installed_size()?;
            println!("Installed packages: {}", count);
            println!("Total size:         {} bytes ({:.1} MiB)",
                size, size as f64 / 1_048_576.0);
        }

        "resync" => {
            let sync = DbSync::new(&db, &paths);
            let n = sync.full_resync()?;
            println!("Resynced {} generation(s).", n);
        }

        other => {
            eprintln!("Unknown command: {}", other);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}
