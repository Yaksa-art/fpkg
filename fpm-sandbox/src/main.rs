use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use fpm_sandbox::{
    bubble::BubbleSandbox,
    overlay::{OverlaySandbox},
    sandbox::{Sandbox, SandboxConfig, SandboxLevel},
};

#[derive(Debug, Parser)]
#[command(name = "fpm-sandbox", about = "M11 User Namespace Manager")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    Enter {
        #[arg(long)] pkg: String,
        #[arg(long, default_value = "overlay")] level: String,
    },
    Leave {
        #[arg(long)] pkg: String,
        #[arg(long, default_value = "overlay")] level: String,
    },
    Remove {
        #[arg(long)] pkg: String,
    },
    Run {
        #[arg(long)] pkg: String,
        #[arg(long, default_value = "bubble")] level: String,
        #[arg(trailing_var_arg = true)] argv: Vec<String>,
    },
    Check,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Enter { pkg, level } => {
            let level: SandboxLevel = level.parse()?;
            let cfg = SandboxConfig::new(&pkg, level);
            match level {
                SandboxLevel::None => {}
                SandboxLevel::Overlay | SandboxLevel::Full => {
                    OverlaySandbox.enter(&cfg)?;
                }
                SandboxLevel::Bubble => {
                    OverlaySandbox.enter(&cfg)?;
                }
            }
        }

        Cmd::Leave { pkg, level } => {
            let level: SandboxLevel = level.parse()?;
            let cfg = SandboxConfig::new(&pkg, level);
            OverlaySandbox.leave(&cfg)?;
        }

        Cmd::Remove { pkg } => {
            let cfg = SandboxConfig::new(&pkg, SandboxLevel::Overlay);
            OverlaySandbox::remove_overlay(&cfg)?;
        }

        Cmd::Run { pkg, level, argv } => {
            let level: SandboxLevel = level.parse()?;
            let cfg = SandboxConfig::new(&pkg, level);
            let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
            let code = match level {
                SandboxLevel::None | SandboxLevel::Overlay => {
                    OverlaySandbox.run(&cfg, &argv_refs)?
                }
                SandboxLevel::Bubble | SandboxLevel::Full => {
                    BubbleSandbox.run(&cfg, &argv_refs)?
                }
            };
            std::process::exit(code);
        }

        Cmd::Check => {
            if fpm_sandbox::ns::check_user_ns_support() {
                println!("user namespaces: supported");
            } else {
                println!("user namespaces: NOT supported");
                std::process::exit(1);
            }
            if which::which("bwrap").is_ok() {
                println!("bwrap: found");
            } else {
                println!("bwrap: NOT found");
            }
            if which::which("fuse-overlayfs").is_ok() {
                println!("fuse-overlayfs: found");
            } else {
                println!("fuse-overlayfs: NOT found");
            }
        }
    }

    Ok(())
}
