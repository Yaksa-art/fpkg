use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
};
use tracing::{debug, info};
use crate::{SandboxConfig, SandboxError, sandbox::Sandbox};

pub struct OverlaySandbox;

impl Sandbox for OverlaySandbox {
    fn enter(&self, cfg: &SandboxConfig) -> Result<(), SandboxError> {
        which_fuse_overlayfs()?;

        for dir in &[
            cfg.lower_dir(),
            cfg.upper_dir(),
            cfg.work_dir(),
            cfg.merge_dir(),
        ] {
            fs::create_dir_all(dir)?;
            fs::set_permissions(dir, fs::Permissions::from_mode(0o755))?;
        }

        let lower = cfg.lower_dir().display().to_string();
        let upper = cfg.upper_dir().display().to_string();
        let work  = cfg.work_dir().display().to_string();
        let merge = cfg.merge_dir().display().to_string();

        let status = Command::new("fuse-overlayfs")
            .args([
                "-o",
                &format!("lowerdir={lower},upperdir={upper},workdir={work}"),
                &merge,
            ])
            .status()
            .map_err(|e| SandboxError::OverlayMount {
                pkg: cfg.pkg_name.clone(),
                source: e,
            })?;

        if !status.success() {
            return Err(SandboxError::OverlayMount {
                pkg: cfg.pkg_name.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("fuse-overlayfs exited with {:?}", status.code()),
                ),
            });
        }

        info!(pkg = %cfg.pkg_name, merge = %merge, "overlay mounted");
        Ok(())
    }

    fn leave(&self, cfg: &SandboxConfig) -> Result<(), SandboxError> {
        let merge = cfg.merge_dir().display().to_string();
        debug!(pkg = %cfg.pkg_name, "unmounting overlay");

        let _ = Command::new("fusermount3")
            .args(["-u", &merge])
            .status()
            .or_else(|_| Command::new("fusermount").args(["-u", &merge]).status())?;

        Ok(())
    }

    fn run(&self, cfg: &SandboxConfig, argv: &[&str]) -> Result<i32, SandboxError> {
        self.enter(cfg)?;
        let result = run_in_dir(&cfg.merge_dir(), argv);
        self.leave(cfg)?;
        result
    }
}

impl OverlaySandbox {
    pub fn remove_overlay(cfg: &SandboxConfig) -> Result<(), SandboxError> {
        let _ = Command::new("fusermount3")
            .args(["-u", &cfg.merge_dir().display().to_string()])
            .status();
        let _ = Command::new("fusermount")
            .args(["-u", &cfg.merge_dir().display().to_string()])
            .status();

        if cfg.overlay_dir.exists() {
            fs::remove_dir_all(&cfg.overlay_dir)?;
            info!(pkg = %cfg.pkg_name, "overlay removed");
        }
        Ok(())
    }
}

fn which_fuse_overlayfs() -> Result<(), SandboxError> {
    which::which("fuse-overlayfs").map_err(|_| SandboxError::FuseOverlayFsMissing)?;
    Ok(())
}

fn run_in_dir(dir: &Path, argv: &[&str]) -> Result<i32, SandboxError> {
    let (prog, args) = argv.split_first().ok_or_else(|| {
        SandboxError::Anyhow(anyhow::anyhow!("empty argv"))
    })?;
    let status = Command::new(prog)
        .args(args)
        .current_dir(dir)
        .status()?;
    Ok(status.code().unwrap_or(1))
}
