use anyhow::Result;
use clap::{Parser, Subcommand};
use fpm_fetcher::{
    cache::Cache,
    fetcher::{fetch_all, fetch_one, FetchRequest},
    mirror::{probe_mirrors, Mirror},
    progress::Progress,
    types::PackageUrl,
};
use std::time::Duration;

#[derive(Parser)]
#[command(name = "fpm-fetcher", about = "M2 Fetcher — download and cache .fpkg files")]
struct Cli {
    #[arg(long, help = "Run in user mode (uses ~/.cache/fpm)")]
    user: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Download {
        #[arg(long)]
        name: String,
        #[arg(long)]
        version: String,
        #[arg(long, num_args = 1..)]
        url: Vec<String>,
        #[arg(long, help = "Expected BLAKE3 hex hash")]
        blake3: Option<String>,
        #[arg(long, default_value = "4")]
        parallel: usize,
    },
    Probe {
        #[arg(num_args = 1..)]
        mirrors: Vec<String>,
    },
    Cached {
        #[arg(long)]
        name: String,
        #[arg(long)]
        version: String,
    },
    Purge {
        #[arg(long)]
        name: String,
        #[arg(long)]
        version: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cache = Cache::from_env(cli.user);

    match cli.command {
        Command::Download { name, version, url, blake3, parallel } => {
            let pkg = PackageUrl { name: name.clone(), version, urls: url, blake3, size: None };
            let progress = Progress::new(pkg.size.unwrap_or(0));
            let req = FetchRequest { package: pkg, cache, parallel, progress: Some(progress.clone()) };
            let result = fetch_one(req).await?;
            if result.from_cache {
                println!("cache hit: {}", result.path.display());
            } else {
                println!("downloaded: {}", result.path.display());
            }
        }
        Command::Probe { mirrors } => {
            let mirror_list: Vec<Mirror> = mirrors.into_iter()
                .enumerate()
                .map(|(i, url)| Mirror::new(url, i as u32))
                .collect();
            let ranked = probe_mirrors(&mirror_list, Duration::from_secs(3)).await;
            if ranked.is_empty() {
                println!("no reachable mirrors");
            } else {
                println!("ranked mirrors:");
                for m in &ranked {
                    println!("  {}", m.url);
                }
            }
        }
        Command::Cached { name, version } => {
            let key = format!("{}-{}", name, version);
            if cache.contains(&key) {
                println!("cached: {}", cache.path_for(&key).display());
            } else {
                println!("not cached");
            }
        }
        Command::Purge { name, version } => {
            let key = format!("{}-{}", name, version);
            cache.remove(&key)?;
            println!("purged: {}-{}", name, version);
        }
    }

    Ok(())
}
