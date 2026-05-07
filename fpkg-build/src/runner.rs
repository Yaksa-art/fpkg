use std::{
    path::Path,
    process::Command,
};
use tracing::{debug, info};
use crate::{BuildError, PkgBuild};
use crate::prepare::BuildEnv;

pub fn run_build(pb: &PkgBuild, env: &BuildEnv) -> Result<(), BuildError> {
    info!(pkg = %pb.package.name, "running build script");

    let mut cmd = build_command(pb, env);
    debug!(cmd = ?cmd, "build command");

    let status = cmd.status()?;
    if !status.success() {
        return Err(BuildError::BuildFailed(status.code().unwrap_or(1)));
    }

    if let Some(pkg_script) = &pb.build.package_install {
        info!(pkg = %pb.package.name, "running package-install script");
        run_script(pkg_script, env)?;
    }

    Ok(())
}

fn build_command(pb: &PkgBuild, env: &BuildEnv) -> Command {
    let use_bwrap = which::which("bwrap").is_ok();

    if use_bwrap {
        let mut cmd = Command::new("bwrap");
        cmd.args([
            "--unshare-user", "--unshare-pid", "--unshare-net",
            "--ro-bind", "/usr", "/usr",
            "--ro-bind", "/lib", "/lib",
            "--ro-bind", "/lib64", "/lib64",
            "--proc", "/proc",
            "--dev", "/dev",
            "--bind", env.build_dir.to_str().unwrap(), "/build",
            "--bind", env.src_dir.to_str().unwrap(),   "/src",
            "--bind", env.destdir.to_str().unwrap(),   "/pkg",
            "--ro-bind", env.script_path.to_str().unwrap(), "/build.sh",
            "--chdir", "/build",
            "--setenv", "SRCDIR",   "/src",
            "--setenv", "DESTDIR",  "/pkg",
            "--setenv", "PKGNAME",  &pb.package.name,
            "--setenv", "PKGVER",   &pb.package.version,
            "--setenv", "PKGREL",   &pb.package.release.to_string(),
            "--setenv", "FPM_BUILD", "1",
        ]);
        cmd.args(["/bin/sh", "/build.sh"]);
        cmd
    } else {
        let mut cmd = Command::new("/bin/sh");
        cmd.arg(env.script_path.to_str().unwrap());
        cmd.current_dir(&env.build_dir);
        cmd.env("SRCDIR",    env.src_dir.to_str().unwrap());
        cmd.env("DESTDIR",   env.destdir.to_str().unwrap());
        cmd.env("PKGNAME",   &pb.package.name);
        cmd.env("PKGVER",    &pb.package.version);
        cmd.env("PKGREL",    &pb.package.release.to_string());
        cmd.env("FPM_BUILD", "1");
        cmd
    }
}

fn run_script(script: &str, env: &BuildEnv) -> Result<(), BuildError> {
    let tmp = tempfile::NamedTempFile::new()?;
    std::fs::write(tmp.path(), script)?;
    let status = Command::new("/bin/sh")
        .arg(tmp.path())
        .current_dir(&env.build_dir)
        .env("DESTDIR", env.destdir.to_str().unwrap())
        .status()?;
    if !status.success() {
        return Err(BuildError::BuildFailed(status.code().unwrap_or(1)));
    }
    Ok(())
}
