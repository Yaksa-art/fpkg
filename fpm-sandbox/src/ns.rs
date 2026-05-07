use nix::sched::{unshare, CloneFlags};
use tracing::debug;
use crate::SandboxError;

pub fn unshare_user_mount() -> Result<(), SandboxError> {
    debug!("unshare CLONE_NEWUSER | CLONE_NEWNS");
    unshare(CloneFlags::CLONE_NEWUSER | CloneFlags::CLONE_NEWNS)
        .map_err(|e| SandboxError::Namespace(e.to_string()))?;
    write_uid_gid_maps()?;
    Ok(())
}

pub fn unshare_all() -> Result<(), SandboxError> {
    debug!("unshare all namespaces");
    unshare(
        CloneFlags::CLONE_NEWUSER
        | CloneFlags::CLONE_NEWNS
        | CloneFlags::CLONE_NEWPID
        | CloneFlags::CLONE_NEWIPC
        | CloneFlags::CLONE_NEWNET,
    )
    .map_err(|e| SandboxError::Namespace(e.to_string()))?;
    write_uid_gid_maps()?;
    Ok(())
}

fn write_uid_gid_maps() -> Result<(), SandboxError> {
    let pid = std::process::id();
    let uid = nix::unistd::getuid();
    let gid = nix::unistd::getgid();

    std::fs::write(
        format!("/proc/{pid}/uid_map"),
        format!("0 {uid} 1\n"),
    )
    .map_err(|e| SandboxError::Namespace(format!("uid_map: {e}")))?;

    std::fs::write(
        format!("/proc/{pid}/setgroups"),
        "deny\n",
    )
    .map_err(|e| SandboxError::Namespace(format!("setgroups: {e}")))?;

    std::fs::write(
        format!("/proc/{pid}/gid_map"),
        format!("0 {gid} 1\n"),
    )
    .map_err(|e| SandboxError::Namespace(format!("gid_map: {e}")))?;

    Ok(())
}

pub fn check_user_ns_support() -> bool {
    std::path::Path::new("/proc/self/ns/user").exists()
}
