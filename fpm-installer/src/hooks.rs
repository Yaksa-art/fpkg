use std::path::Path;
use fpm_hooks::{
    runner::{HookKind, Runner, RunnerConfig},
    HookError,
};
use crate::error::InstallerError;

fn runner() -> Runner {
    Runner::new(RunnerConfig::default())
}

pub fn run_pre_install(root: &Path, name: &str, version: &str) -> Result<(), InstallerError> {
    run(root, name, version, HookKind::PreInstall)
}

pub fn run_post_install(root: &Path, name: &str, version: &str) -> Result<(), InstallerError> {
    run(root, name, version, HookKind::PostInstall)
}

pub fn run_pre_remove(root: &Path, name: &str, version: &str) -> Result<(), InstallerError> {
    run(root, name, version, HookKind::PreRemove)
}

pub fn run_post_remove(root: &Path, name: &str, version: &str) -> Result<(), InstallerError> {
    run(root, name, version, HookKind::PostRemove)
}

fn run(root: &Path, name: &str, version: &str, kind: HookKind) -> Result<(), InstallerError> {
    runner()
        .run(root, name, version, kind)
        .map_err(|e| match e {
            HookError::ScriptFailed { script, code, .. } => {
                InstallerError::ScriptFailed { script, code }
            }
            HookError::Timeout { script, secs } => {
                InstallerError::ScriptTimeout { script, secs }
            }
            HookError::Io(io) => InstallerError::Io(io),
            other => InstallerError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                other.to_string(),
            )),
        })?;
    Ok(())
}
