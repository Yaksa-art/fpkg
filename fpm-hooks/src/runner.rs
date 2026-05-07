use std::{
    io::Read,
    path::Path,
    time::{Duration, Instant},
};
use crate::{
    env::HookEnv,
    error::HookError,
    sandbox::{bwrap_available, plain_command, sandboxed_command, which_bwrap},
};

pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookKind {
    PreInstall,
    PostInstall,
    PreRemove,
    PostRemove,
}

impl HookKind {
    pub fn script_name(self) -> &'static str {
        match self {
            Self::PreInstall  => "pre-install.sh",
            Self::PostInstall => "post-install.sh",
            Self::PreRemove   => "pre-remove.sh",
            Self::PostRemove  => "post-remove.sh",
        }
    }
}

#[derive(Debug)]
pub struct HookResult {
    pub hook: &'static str,
    pub sandboxed: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub timeout_secs: u64,
    pub sandbox: SandboxMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxMode {
    Auto,
    Bwrap,
    Plain,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            sandbox: SandboxMode::Auto,
        }
    }
}

pub struct Runner {
    config: RunnerConfig,
}

impl Runner {
    pub fn new(config: RunnerConfig) -> Self {
        Self { config }
    }

    pub fn default() -> Self {
        Self { config: RunnerConfig::default() }
    }

    pub fn run(
        &self,
        root: &Path,
        name: &str,
        version: &str,
        kind: HookKind,
    ) -> Result<Option<HookResult>, HookError> {
        let script_path = root
            .join("var/lib/fpm/hooks")
            .join(format!("{}-{}", name, version))
            .join(kind.script_name());

        if !script_path.exists() {
            return Ok(None);
        }

        set_executable(&script_path)?;

        let env = HookEnv::new(root, name, version, kind.script_name());
        let sandboxed = self.should_sandbox();

        tracing::info!(
            "hook: {} {}-{} [{}]",
            kind.script_name(), name, version,
            if sandboxed { "bwrap" } else { "plain" }
        );

        let mut cmd = if sandboxed {
            let bwrap = which_bwrap().unwrap();
            sandboxed_command(&bwrap, &script_path, root, &env)
        } else {
            plain_command(&script_path, root, &env)
        };

        let mut child = cmd.spawn()
            .map_err(|e| HookError::BwrapExec(e.to_string()))?;

        let timeout = Duration::from_secs(self.config.timeout_secs);
        let start = Instant::now();

        loop {
            match child.try_wait()? {
                Some(status) => {
                    let stdout = read_output(child.stdout.take());
                    let stderr = read_output(child.stderr.take());

                    if !status.success() {
                        return Err(HookError::ScriptFailed {
                            script: kind.script_name().to_string(),
                            code: status.code().unwrap_or(-1),
                            stdout,
                            stderr,
                        });
                    }

                    return Ok(Some(HookResult {
                        hook: kind.script_name(),
                        sandboxed,
                        stdout,
                        stderr,
                    }));
                }
                None => {
                    if start.elapsed() >= timeout {
                        let _ = child.kill();
                        return Err(HookError::Timeout {
                            script: kind.script_name().to_string(),
                            secs: self.config.timeout_secs,
                        });
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }

    fn should_sandbox(&self) -> bool {
        match self.config.sandbox {
            SandboxMode::Bwrap => true,
            SandboxMode::Plain => false,
            SandboxMode::Auto => {
                let available = bwrap_available();
                if !available {
                    tracing::warn!("hook: bwrap not found — running unsandboxed");
                }
                available
            }
        }
    }
}

fn set_executable(path: &Path) -> Result<(), HookError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

fn read_output(source: Option<impl Read>) -> String {
    match source {
        None => String::new(),
        Some(mut r) => {
            let mut buf = String::new();
            let _ = r.read_to_string(&mut buf);
            buf
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn write_script(dir: &Path, name: &str, version: &str, hook: &str, body: &str) {
        let hook_dir = dir
            .join("var/lib/fpm/hooks")
            .join(format!("{}-{}", name, version));
        fs::create_dir_all(&hook_dir).unwrap();
        let path = hook_dir.join(hook);
        fs::write(&path, body).unwrap();
    }

    #[test]
    fn no_script_returns_none() {
        let tmp = TempDir::new().unwrap();
        let runner = Runner::new(RunnerConfig {
            sandbox: SandboxMode::Plain,
            ..Default::default()
        });
        let result = runner.run(tmp.path(), "mypkg", "1.0.0", HookKind::PreInstall);
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn successful_plain_hook() {
        let tmp = TempDir::new().unwrap();
        write_script(tmp.path(), "mypkg", "1.0.0", "pre-install.sh", "#!/bin/sh\nexit 0\n");
        let runner = Runner::new(RunnerConfig {
            sandbox: SandboxMode::Plain,
            ..Default::default()
        });
        let result = runner.run(tmp.path(), "mypkg", "1.0.0", HookKind::PreInstall);
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn failing_hook_returns_error() {
        let tmp = TempDir::new().unwrap();
        write_script(tmp.path(), "mypkg", "1.0.0", "post-install.sh", "#!/bin/sh\nexit 42\n");
        let runner = Runner::new(RunnerConfig {
            sandbox: SandboxMode::Plain,
            ..Default::default()
        });
        let err = runner.run(tmp.path(), "mypkg", "1.0.0", HookKind::PostInstall).unwrap_err();
        match err {
            HookError::ScriptFailed { code, .. } => assert_eq!(code, 42),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn timeout_kills_script() {
        let tmp = TempDir::new().unwrap();
        write_script(tmp.path(), "mypkg", "1.0.0", "pre-install.sh", "#!/bin/sh\nsleep 10\n");
        let runner = Runner::new(RunnerConfig {
            timeout_secs: 1,
            sandbox: SandboxMode::Plain,
        });
        let err = runner.run(tmp.path(), "mypkg", "1.0.0", HookKind::PreInstall).unwrap_err();
        assert!(matches!(err, HookError::Timeout { .. }));
    }
}
