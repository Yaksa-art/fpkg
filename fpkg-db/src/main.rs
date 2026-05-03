use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use fpkg_db::{
    Database,
    models::{GenerationEntry, NewPackage, NewRepo},
};
use fpkg_db::{files, generations, hold, packages, repos};

#[derive(Parser)]
#[command(
    name = "fpkg-db",
    version = "0.1.2",
    about = "M8 Local Database — inspect and manage the fpkg package database"
)]
struct Cli {
    #[arg(long, help = "Path to database file (overrides FPM_DB env and auto-detection)")]
    db: Option<PathBuf>,

    #[arg(long, help = "Use user-mode database (~/.local/share/fpm/db.sqlite)")]
    user: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Initialize database and print its path")]
    Init,

    #[command(about = "Show database statistics")]
    Stats,

    #[command(about = "List installed packages")]
    List {
        #[arg(long, help = "Filter by mode: system | user")]
        mode: Option<String>,
    },

    #[command(about = "Show details for a single package")]
    Info {
        #[arg(help = "Package name")]
        name: String,
        #[arg(long, default_value = "system")]
        mode: String,
    },

    #[command(about = "Search installed packages by name or summary")]
    Search {
        #[arg(help = "Search query")]
        query: String,
    },

    #[command(about = "Show which package owns a file path")]
    Owns {
        #[arg(help = "File path to query")]
        path: String,
    },

    #[command(about = "List files belonging to a package")]
    Files {
        #[arg(help = "Package name")]
        name: String,
        #[arg(long, default_value = "system")]
        mode: String,
    },

    #[command(about = "List transaction generations")]
    Generations {
        #[arg(long, default_value = "20", help = "How many to show")]
        limit: usize,
    },

    #[command(about = "Record a new generation (internal use)")]
    GenRecord {
        #[arg(long, help = "Action label (install/remove/upgrade)")]
        action: String,
        #[arg(long, help = "Note to attach")]
        note: Option<String>,
        #[arg(long, help = "Package entries as 'name:version:action'", num_args = 1..)]
        pkg: Vec<String>,
    },

    #[command(about = "List configured repositories")]
    Repos,

    #[command(about = "Add a repository")]
    RepoAdd {
        #[arg(long, required = true)]
        name: String,
        #[arg(long, required = true)]
        url: String,
        #[arg(long, default_value = "fpkg")]
        repo_type: String,
        #[arg(long, default_value = "50")]
        priority: i64,
        #[arg(long, default_value = "")]
        suite: String,
        #[arg(long, default_value = "")]
        components: String,
    },

    #[command(about = "Remove a repository by name")]
    RepoRemove {
        name: String,
    },

    #[command(about = "List held packages")]
    Holds,

    #[command(about = "Hold a package at its current version")]
    HoldAdd {
        name: String,
        #[arg(long, default_value = "")]
        version: String,
        #[arg(long, default_value = "")]
        reason: String,
    },

    #[command(about = "Unhold a package")]
    HoldRemove {
        name: String,
    },

    #[command(about = "Register a package as installed (internal use)")]
    Register {
        #[arg(long, required = true)] name: String,
        #[arg(long, required = true)] version: String,
        #[arg(long, default_value = "1")] release: u32,
        #[arg(long, default_value = "x86_64")] arch: String,
        #[arg(long, default_value = "system")] mode: String,
        #[arg(long, default_value = "")] summary: String,
        #[arg(long, default_value = "")] license: String,
        #[arg(long, default_value = "")] maintainer: String,
        #[arg(long, default_value = "")] homepage: String,
        #[arg(long, default_value = "0")] install_size: i64,
        #[arg(long, default_value = "native")] origin_format: String,
        #[arg(long, default_value = "")] manifest_hash: String,
        #[arg(long, default_value = "")] content_tree: String,
    },

    #[command(about = "Unregister a package (internal use)")]
    Unregister {
        name: String,
        #[arg(long, default_value = "system")]
        mode: String,
    },
}

fn open_db(cli: &Cli) -> Result<Database> {
    if let Some(ref path) = cli.db {
        return Database::open(path);
    }
    Database::open_default(cli.user)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let db = open_db(&cli)?;

    match &cli.command {
        Command::Init => {
            println!("[✓] Database initialized: {}", db.path.display());
            let stats = db.stats()?;
            println!("    Packages    : {}", stats.packages);
            println!("    Files       : {}", stats.files);
            println!("    Generations : {}", stats.generations);
            println!("    Repos       : {}", stats.repos);
            println!("    Holds       : {}", stats.holds);
        }

        Command::Stats => {
            let s = db.stats()?;
            println!("Database: {}", db.path.display());
            println!("Packages    : {}", s.packages);
            println!("Files       : {}", s.files);
            println!("Generations : {}", s.generations);
            println!("Repos       : {}", s.repos);
            println!("Holds       : {}", s.holds);
        }

        Command::List { mode } => {
            let pkgs = packages::list_all(&db.conn, mode.as_deref())?;
            if pkgs.is_empty() {
                println!("No packages installed.");
                return Ok(());
            }
            println!("{:<30} {:<16} {:<6} {:<8} {}", "NAME", "VERSION", "REL", "MODE", "SUMMARY");
            println!("{}", "-".repeat(90));
            for p in &pkgs {
                println!(
                    "{:<30} {:<16} {:<6} {:<8} {}",
                    p.name, p.version, p.release, p.mode, p.summary
                );
            }
            println!("\nTotal: {}", pkgs.len());
        }

        Command::Info { name, mode } => {
            match packages::get_by_name(&db.conn, name, mode)? {
                None => {
                    eprintln!("[✗] Package not found: {} (mode={})", name, mode);
                    std::process::exit(1);
                }
                Some(p) => {
                    let file_count = files::count_for_package(&db.conn, p.id)?;
                    let held = hold::is_held(&db.conn, name)?;
                    println!("Name          : {}", p.name);
                    println!("Version       : {}-{}", p.version, p.release);
                    println!("Arch          : {}", p.arch);
                    println!("Mode          : {}", p.mode);
                    println!("Summary       : {}", p.summary);
                    println!("License       : {}", p.license);
                    println!("Maintainer    : {}", p.maintainer);
                    println!("Homepage      : {}", p.homepage);
                    println!("Install size  : {} bytes", p.install_size);
                    println!("Origin        : {}", p.origin_format);
                    println!("Installed at  : {}", p.install_date);
                    println!("Files tracked : {}", file_count);
                    println!("Held          : {}", if held { "yes" } else { "no" });
                    if !p.manifest_hash.is_empty() {
                        println!("Manifest hash : {}...", &p.manifest_hash[..p.manifest_hash.len().min(48)]);
                    }
                }
            }
        }

        Command::Search { query } => {
            let pkgs = packages::search(&db.conn, query)?;
            if pkgs.is_empty() {
                println!("No results for: {}", query);
                return Ok(());
            }
            for p in &pkgs {
                println!("{} ({}-{}) [{}] — {}", p.name, p.version, p.release, p.mode, p.summary);
            }
        }

        Command::Owns { path } => {
            match files::owner_of(&db.conn, path)? {
                None => println!("{}: not owned by any package", path),
                Some((name, version, mode)) => {
                    println!("{}: owned by {} {} [{}]", path, name, version, mode);
                }
            }
        }

        Command::Files { name, mode } => {
            match packages::get_by_name(&db.conn, name, mode)? {
                None => {
                    eprintln!("[✗] Package not found: {}", name);
                    std::process::exit(1);
                }
                Some(p) => {
                    let fs = files::list_for_package(&db.conn, p.id)?;
                    for f in &fs {
                        let flag = if f.is_config { " [config]" } else { "" };
                        println!("{}{}", f.path, flag);
                    }
                    println!("\nTotal: {} file(s)", fs.len());
                }
            }
        }

        Command::Generations { limit } => {
            let gens = generations::list(&db.conn, *limit)?;
            if gens.is_empty() {
                println!("No generations recorded.");
                return Ok(());
            }
            println!("{:<6} {:<22} {:<12} {}", "GEN", "DATE", "ACTION", "CHANGES");
            println!("{}", "-".repeat(75));
            for g in &gens {
                let rb = if g.rolled_back { " [rolled back]" } else { "" };
                let changes: Vec<String> = g.packages.iter()
                    .map(|e| format!("{} {} ({})", e.action, e.name, e.version))
                    .collect();
                println!(
                    "#{:<5} {:<22} {:<12} {}{}",
                    g.id, g.created_at, g.action,
                    changes.join(", "),
                    rb,
                );
            }
        }

        Command::GenRecord { action, note, pkg } => {
            let entries: Vec<GenerationEntry> = pkg.iter().filter_map(|s| {
                let parts: Vec<&str> = s.splitn(3, ':').collect();
                if parts.len() == 3 {
                    Some(GenerationEntry {
                        name:    parts[0].to_string(),
                        version: parts[1].to_string(),
                        action:  parts[2].to_string(),
                    })
                } else {
                    None
                }
            }).collect();
            let id = generations::record(
                &db.conn, action, &entries,
                note.as_deref().unwrap_or(""),
            )?;
            println!("[✓] Generation #{} recorded", id);
        }

        Command::Repos => {
            let rs = repos::list(&db.conn)?;
            if rs.is_empty() {
                println!("No repositories configured.");
                return Ok(());
            }
            println!("{:<20} {:<10} {:<6} {:<8} {}", "NAME", "TYPE", "PRIO", "ENABLED", "URL");
            println!("{}", "-".repeat(80));
            for r in &rs {
                println!(
                    "{:<20} {:<10} {:<6} {:<8} {}",
                    r.name, r.repo_type, r.priority,
                    if r.enabled { "yes" } else { "no" },
                    r.url
                );
            }
        }

        Command::RepoAdd { name, url, repo_type, priority, suite, components } => {
            repos::add(&db.conn, &NewRepo {
                name: name.clone(),
                url: url.clone(),
                repo_type: repo_type.clone(),
                enabled: true,
                priority: *priority,
                pubkey: String::new(),
                suite: suite.clone(),
                components: components.clone(),
            })?;
            println!("[✓] Repository '{}' added", name);
        }

        Command::RepoRemove { name } => {
            if repos::remove(&db.conn, name)? {
                println!("[✓] Repository '{}' removed", name);
            } else {
                eprintln!("[✗] Repository not found: {}", name);
                std::process::exit(1);
            }
        }

        Command::Holds => {
            let hs = hold::list(&db.conn)?;
            if hs.is_empty() {
                println!("No packages on hold.");
                return Ok(());
            }
            println!("{:<30} {:<16} {}", "PACKAGE", "VERSION", "REASON");
            println!("{}", "-".repeat(70));
            for h in &hs {
                println!("{:<30} {:<16} {}", h.name, h.version, h.reason);
            }
        }

        Command::HoldAdd { name, version, reason } => {
            hold::add(&db.conn, name, version, reason)?;
            println!("[✓] {} is now held", name);
        }

        Command::HoldRemove { name } => {
            if hold::remove(&db.conn, name)? {
                println!("[✓] {} removed from hold", name);
            } else {
                eprintln!("[✗] Package not found in hold: {}", name);
                std::process::exit(1);
            }
        }

        Command::Register { name, version, release, arch, mode, summary, license,
                            maintainer, homepage, install_size, origin_format,
                            manifest_hash, content_tree } => {
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let id = packages::upsert(&db.conn, &NewPackage {
                name: name.clone(),
                version: version.clone(),
                release: *release,
                arch: arch.clone(),
                mode: mode.clone(),
                summary: summary.clone(),
                license: license.clone(),
                maintainer: maintainer.clone(),
                homepage: homepage.clone(),
                install_size: *install_size,
                origin_format: origin_format.clone(),
                install_date: now,
                manifest_hash: manifest_hash.clone(),
                content_tree: content_tree.clone(),
            })?;
            println!("[✓] Registered {} {} (id={})", name, version, id);
        }

        Command::Unregister { name, mode } => {
            if packages::remove(&db.conn, name, mode)? {
                println!("[✓] Unregistered {}", name);
            } else {
                eprintln!("[✗] Package not found: {} (mode={})", name, mode);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
