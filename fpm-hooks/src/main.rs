use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use fpm_hooks::{
    runner::{HookKind, Runner, RunnerConfig, SandboxMode},
    sandbox::bwrap_available,
};

#[derive(Parser)]
#[command(name = "fpm-hooks", about = "M7 — sandboxed hook runner", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Run {
        #[arg(long)] root: String,
        #[arg(long)] name: String,
        #[arg(long)] version: String,
        #[arg(long, value_enum)] hook: HookArg,
        #[arg(long)] no_sandbox: bool,
        #[arg(long, default_value_t = 60)] timeout: u64,
    },
    Check,
}

#[derive(clap::ValueEnum, Clone)]
enum HookArg {
    PreInstall,
    PostInstall,
    PreRemove,
    PostRemove,
}

impl From<HookArg> for HookKind {
    fn from(a: HookArg) -> Self {
        match a {
            HookArg::PreInstall  => HookKind::PreInstall,
            HookArg::PostInstall => HookKind::PostInstall,
            HookArg::PreRemove   => HookKind::PreRemove,
            HookArg::PostRemove  => HookKind::PostRemove,
        }
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Run { root, name, version, hook, no_sandbox, timeout } => {
            let config = RunnerConfig {
                timeout_secs: timeout,
                sandbox: if no_sandbox { SandboxMode::Plain } else { SandboxMode::Auto },
            };
            let runner = Runner::new(config);
            let kind: HookKind = hook.into();
            match runner.run(std::path::Path::new(&root), &name, &version, kind)? {
                None => println!("no hook script found — skipped"),
                Some(r) => {
                    println!("ok [{}]", if r.sandboxed { "bwrap" } else { "plain" });
                    if !r.stdout.is_empty() { print!("{}", r.stdout); }
                    if !r.stderr.is_empty() { eprint!("{}", r.stderr); }
                }
            }
        }
        Cmd::Check => {
            if bwrap_available() {
                println!("bwrap: available");
            } else {
                println!("bwrap: not found — hooks will run unsandboxed");
            }
        }
    }

    Ok(())
}
