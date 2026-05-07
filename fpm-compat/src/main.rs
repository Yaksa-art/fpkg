use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;
use fpm_compat::convert::{convert, to_manifest_toml};

#[derive(Debug, Parser)]
#[command(
    name  = "fconv",
    about = "Convert foreign packages (.deb/.rpm/.apk/.pkg.tar.zst) to .fpkg manifest"
)]
struct Cli {
    #[arg(help = "Input package file")]
    input: PathBuf,

    #[arg(short, long, help = "Output manifest.toml path (default: stdout)")]
    output: Option<PathBuf>,

    #[arg(long, help = "Print parsed fields as JSON")]
    json: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let pkg = convert(&cli.input)
        .with_context(|| format!("converting {:?}", cli.input))?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&pkg)?);
        return Ok(());
    }

    let manifest = to_manifest_toml(&pkg);

    match cli.output {
        Some(out) => {
            std::fs::write(&out, &manifest)
                .with_context(|| format!("writing to {:?}", out))?;
            eprintln!("wrote manifest to {}", out.display());
        }
        None => print!("{manifest}"),
    }

    Ok(())
}
