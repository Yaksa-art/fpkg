use anyhow::Result;
use clap::Parser;
use std::{
    path::PathBuf,
    sync::Arc,
};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod config;
mod handlers;
mod rpc;
mod socket;
mod state;

use state::DaemonState;

#[derive(Debug, Parser)]
#[command(name = "fpmd", about = "fpm daemon")]
struct Cli {
    #[arg(long, help = "Config file path")]
    config: Option<PathBuf>,

    #[arg(long, help = "Socket path override")]
    socket: Option<PathBuf>,

    #[arg(long, help = "Run in foreground (don't daemonize)")]
    foreground: bool,

    #[arg(long, default_value = "system", help = "Mode: system or user")]
    mode: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("fpmd=info".parse().unwrap()))
        .init();

    let cli = Cli::parse();

    let cfg = config::load(cli.config.as_deref(), &cli.mode)?;
    info!(socket = %cfg.socket_path.display(), mode = %cli.mode, "fpmd starting");

    let state = Arc::new(DaemonState::new(cfg.clone())?)
        as Arc<DaemonState>;

    if let Some(parent) = cfg.socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _ = std::fs::remove_file(&cfg.socket_path);

    socket::run_accept_loop(cfg.socket_path.clone(), Arc::clone(&state))?;

    Ok(())
}
