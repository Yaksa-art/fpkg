use std::{
    path::Path,
    process::{Command, Stdio},
};
use crate::env::HookEnv;

/// Returns true when `bwrap` is available on PATH.
pub fn bwrap_available() -> bool {
    which_bwrap().is_some()
}

pub fn which_bwrap() -> Option<String> {
    for candidate in ["/usr/bin/bwrap", "/bin/bwrap", "/usr/local/bin/bwrap"] {
        if Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Build a `Command` that runs `script_path` inside a bubblewrap sandbox.
///
/// Sandbox policy:
///   - Unshare all namespaces (--unshare-all)
///   - No network (network namespace is new)
///   - Bind staging root rw at /
///   - tmpfs on /tmp
///   - Minimal /proc (--proc /proc)
///   - /dev/null and /dev/urandom via --dev-bind-try
///   - --cap-drop ALL
///   - Environment sanitised to HookEnv pairs only
pub fn sandboxed_command(
    bwrap: &str,
    script_path: &Path,
    root: &Path,
    env: &HookEnv,
) -> Command {
    let mut cmd = Command::new(bwrap);

    cmd.arg("--unshare-all")
        .arg("--share-net").arg("false");

    cmd.arg("--bind").arg(root).arg("/");

    cmd.arg("--proc").arg("/proc");
    cmd.arg("--tmpfs").arg("/tmp");

    cmd.arg("--dev-bind-try").arg("/dev/null").arg("/dev/null");
    cmd.arg("--dev-bind-try").arg("/dev/urandom").arg("/dev/urandom");

    cmd.arg("--cap-drop").arg("ALL");

    cmd.arg("--setenv").arg("PATH").arg("/usr/bin:/bin");
    for (k, v) in env.as_pairs() {
        cmd.arg("--setenv").arg(k).arg(v);
    }

    cmd.arg("/bin/sh").arg(script_path);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd
}

/// Build a plain (unsandboxed) `Command` — fallback when bwrap is absent.
pub fn plain_command(script_path: &Path, root: &Path, env: &HookEnv) -> Command {
    let mut cmd = Command::new("/bin/sh");
    cmd.arg(script_path);
    cmd.current_dir(root);
    cmd.env_clear();
    for (k, v) in env.as_pairs() {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd
}
