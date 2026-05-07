use std::{
    os::unix::process::CommandExt,
    path::PathBuf,
    process::Command,
};
use tracing::debug;
use crate::{SandboxConfig, SandboxError, sandbox::Sandbox};

pub struct BubbleSandbox;

impl Sandbox for BubbleSandbox {
    fn enter(&self, _cfg: &SandboxConfig) -> Result<(), SandboxError> {
        Ok(())
    }

    fn leave(&self, _cfg: &SandboxConfig) -> Result<(), SandboxError> {
        Ok(())
    }

    fn run(&self, cfg: &SandboxConfig, argv: &[&str]) -> Result<i32, SandboxError> {
        which::which("bwrap").map_err(|_| SandboxError::BwrapMissing)?;

        let mut cmd = Command::new("bwrap");
        build_bwrap_args(&mut cmd, cfg);
        cmd.args(argv);

        debug!(pkg = %cfg.pkg_name, "launching bwrap sandbox");
        let status = cmd.status()?;
        Ok(status.code().unwrap_or(1))
    }
}

pub fn build_bwrap_args(cmd: &mut Command, cfg: &SandboxConfig) {
    cmd.args(["--unshare-user", "--unshare-pid", "--unshare-ipc"]);

    if !cfg.network {
        cmd.arg("--unshare-net");
    }

    cmd.args(["--ro-bind", "/usr", "/usr"]);
    cmd.args(["--ro-bind", "/lib", "/lib"]);
    cmd.args(["--ro-bind", "/lib64", "/lib64"]);
    cmd.args(["--proc", "/proc"]);
    cmd.args(["--dev", "/dev"]);
    cmd.args(["--tmpfs", "/tmp"]);

    if cfg.overlay_dir.exists() {
        let merge = cfg.merge_dir().display().to_string();
        cmd.args(["--bind", &merge, "/run/fpm/pkg"]);
    }

    for path in &cfg.read_only_paths {
        let p = path.display().to_string();
        cmd.args(["--ro-bind", &p, &p]);
    }

    for (host, guest) in &cfg.bind_paths {
        cmd.args(["--bind",
            &host.display().to_string(),
            &guest.display().to_string()
        ]);
    }

    cmd.args(["--setenv", "FPM_SANDBOX", "1"]);
    cmd.args(["--setenv", "FPM_PKG", &cfg.pkg_name]);
}

pub fn bwrap_ro_bind_many(paths: &[PathBuf]) -> Vec<String> {
    paths.iter().flat_map(|p| {
        let s = p.display().to_string();
        vec!["--ro-bind".to_string(), s.clone(), s]
    }).collect()
}
