//! Pre/post-install script runner.
//!
//! Scripts live inside the *extracted staging root* at:
//!   <root>/var/lib/fpm/hooks/<name>-<version>/pre-install.sh
//!   <root>/var/lib/fpm/hooks/<name>-<version>/post-install.sh
//!
//! They are copied there by `extract_data()` from META/scripts/ during extraction.
//!
//! Execution environment:
//!   - Runs as the current user (root for system installs)
//!   - CWD = staging root
//!   - Timeout: 60 s by default
//!   - FPKG_ROOT   = staging root path
//!   - FPKG_NAME   = package name
//!   - FPKG_VERSION= package version
//!
//! In a future M7 (Hooks), scripts will be sandboxed via bwrap/seccomp.
//! For now they run in a plain child process.

use std::{
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
use crate::error::InstallerError;

pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Run pre-install script if present.
pub fn run_pre_install(
    root: &Path,
    name: &str,
    version: &str,
) -> Result<(), InstallerError> {
    run_hook(root, name, version, "pre-install.sh")
}

/// Run post-install script if present.
pub fn run_post_install(
    root: &Path,
    name: &str,
    version: &str,
) -> Result<(), InstallerError> {
    run_hook(root, name, version, "post-install.sh")
}

fn run_hook(
    root: &Path,
    name: &str,
    version: &str,
    script_name: &str,
) -> Result<(), InstallerError> {
    let script_path = root
        .join("var/lib/fpm/hooks")
        .join(format!("{}-{}", name, version))
        .join(script_name);

    if !script_path.exists() {
        return Ok(()); // no hook — fine
    }

    tracing::info!("Running hook: {}", script_path.display());

    // Make executable
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&script_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script_path, perms)?;

    let mut child = Command::new("/bin/sh")
        .arg(&script_path)
        .env("FPKG_ROOT", root)
        .env("FPKG_NAME", name)
        .env("FPKG_VERSION", version)
        .current_dir(root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    // Manual timeout loop (no nix/tokio dependency)
    let timeout = Duration::from_secs(DEFAULT_TIMEOUT_SECS);
    let start = std::time::Instant::now();
    loop {
        match child.try_wait()? {
            Some(status) => {
                if !status.success() {
                    let code = status.code().unwrap_or(-1);
                    return Err(InstallerError::ScriptFailed {
                        script: script_name.to_string(),
                        code,
                    });
                }
                return Ok(());
            }
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return Err(InstallerError::ScriptTimeout {
                        script: script_name.to_string(),
                        secs: DEFAULT_TIMEOUT_SECS,
                    });
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
