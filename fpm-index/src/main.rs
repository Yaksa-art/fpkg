use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use fpm_db::pool::{open_pool, open_pool_system};
use fpm_index::{IndexSyncer, SyncOutcome};
use fpm_index::store::IndexStore;

#[derive(Parser)]
#[command(name = "fpm-index", about = "M6 — repo index sync", version)]
struct Cli {
    #[arg(long, env = "FPM_DB")]
    db: Option<String>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Sync {
        #[arg(long)]
        repo: Option<String>,
    },
    List,
    Inspect {
        repo: String,
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let pool = match &cli.db {
        Some(path) => open_pool(path)?,
        None => open_pool_system()?,
    };

    let syncer = IndexSyncer::system();

    match cli.cmd {
        Cmd::Sync { repo } => {
            if let Some(name) = repo {
                let repos = fpm_db::repos::RepoStore::new(&pool);
                let r = repos
                    .get(&name)?
                    .ok_or_else(|| anyhow::anyhow!("repo '{}' not found in db", name))?;
                let outcome = syncer.sync_repo(&r.name, &r.url, r.etag.as_deref(), &pool).await?;
                print_outcome(&name, &outcome);
            } else {
                let results = syncer.sync_all(&pool).await;
                if results.is_empty() {
                    println!("no enabled repos configured");
                }
                for (name, result) in results {
                    match result {
                        Ok(o) => print_outcome(&name, &o),
                        Err(e) => eprintln!("error: {}: {}", name, e),
                    }
                }
            }
        }
        Cmd::List => {
            let store = IndexStore::system();
            let names = store.list()?;
            if names.is_empty() {
                println!("no local index files found");
            } else {
                for name in &names {
                    let idx = store.load(name)?;
                    if let Some(ri) = idx {
                        println!("{:<24} {:>6} packages  generated {}", name, ri.packages.len(), ri.generated_at);
                    }
                }
            }
        }
        Cmd::Inspect { repo, json } => {
            let store = IndexStore::system();
            match store.load(&repo)? {
                None => eprintln!("no index for repo '{}'", repo),
                Some(ri) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&ri)?);
                    } else {
                        println!("repo:          {}", ri.repo);
                        println!("generated_at:  {}", ri.generated_at);
                        println!("packages:      {}", ri.packages.len());
                        println!();
                        for pkg in &ri.packages {
                            println!("  {:<32} {}", pkg.name, pkg.version);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn print_outcome(name: &str, outcome: &SyncOutcome) {
    let label = match outcome {
        SyncOutcome::Created => "created",
        SyncOutcome::Updated => "updated",
        SyncOutcome::NotModified => "up-to-date",
    };
    println!("{}: {}", name, label);
}
