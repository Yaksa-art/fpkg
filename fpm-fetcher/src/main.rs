use std::path::PathBuf;
use fpm_fetcher::{fetch_packages, progress::progress_channel, FetcherConfig};
use fpm_solver::ResolvedPackage;

/// Minimal CLI for fpm-fetcher.
/// In production, the daemon (fpmd) calls fetch_packages() directly.
///
/// Usage:
///   fpm-fetcher fetch <name> <version> [--pubkey <path>] [--config <path>]
///   fpm-fetcher probe-mirrors [--config <path>]

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: fpm-fetcher <command> [args]");
        eprintln!("Commands:");
        eprintln!("  fetch <name> <version> [--pubkey <path>]");
        eprintln!("  probe-mirrors");
        std::process::exit(1);
    }

    let config = FetcherConfig::load_system()
        .or_else(|_| FetcherConfig::load_user())
        .unwrap_or_else(|_| FetcherConfig::default_system());

    match args[1].as_str() {
        "fetch" => {
            if args.len() < 4 {
                eprintln!("Usage: fpm-fetcher fetch <name> <version> [--pubkey <path>]");
                std::process::exit(1);
            }
            let name = args[2].clone();
            let version_str = args[3].clone();

            let pubkey_idx = args.iter().position(|a| a == "--pubkey");
            let pubkey = pubkey_idx
                .and_then(|i| args.get(i + 1))
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/etc/fpm/keys/repo.pub"));

            // Build a minimal ResolvedPackage from CLI args
            let pkg = ResolvedPackage {
                name: name.clone(),
                version: version_str.parse().unwrap_or_default(),
                blake3: None,
                deps: vec![],
            };

            let (tx, mut rx) = progress_channel(64);

            // Print progress events
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    println!("{}", serde_json::to_string(&event).unwrap_or_default());
                }
            });

            let results = fetch_packages(&[pkg], &config, &pubkey, Some(tx)).await;
            for r in results {
                match r {
                    Ok(f) => println!("OK: {} -> {:?}", f.package, f.path),
                    Err(e) => {
                        eprintln!("FAIL: {}", e);
                        std::process::exit(2);
                    }
                }
            }
        }

        "probe-mirrors" => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(config.timeout_secs))
                .user_agent("fpm/0.1.0")
                .build()?;
            let mirrors: Vec<_> = config
                .mirrors
                .iter()
                .filter(|m| m.enabled)
                .map(fpm_fetcher::Mirror::from_config)
                .collect();
            let ranked = fpm_fetcher::mirror::probe_mirrors(mirrors, &client).await;
            for m in &ranked {
                println!("{:>4}ms  {}  {}", m.latency_ms, m.priority, m.name);
            }
        }

        cmd => {
            eprintln!("Unknown command: {}", cmd);
            std::process::exit(1);
        }
    }

    Ok(())
}
