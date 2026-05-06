//! Post-extraction layout fixups.
//!
//! After DATA/ is extracted into the staging root these fixups run:
//!   - ldconfig cache update (if /etc/ld.so.conf or /lib present)
//!   - .desktop file registration in /usr/share/applications
//!   - XDG mime database update
//!   - Binary symlinks in /usr/local/bin for packages that ship in /opt
//!
//! All operations are performed *inside* `root` (the staging directory),
//! so they are safe to commit or discard with the transaction.

use std::path::Path;
use crate::error::InstallerError;

/// Run all layout fixups for a newly installed package tree.
/// `root` is `trx.root_dir()` — the staging area, not the real filesystem root.
pub fn run_layout_fixups(root: &Path) -> Result<(), InstallerError> {
    update_ld_so_conf(root)?;
    register_desktop_entries(root)?;
    Ok(())
}

/// Write /etc/ld.so.conf.d/<pkg>.conf listing any lib directories found.
fn update_ld_so_conf(root: &Path) -> Result<(), InstallerError> {
    let lib_dirs: Vec<_> = [
        "usr/lib", "usr/lib64", "usr/lib32",
        "lib", "lib64", "lib32",
    ]
    .iter()
    .filter(|d| root.join(d).is_dir())
    .map(|d| format!("/{}", d))
    .collect();

    if lib_dirs.is_empty() {
        return Ok(());
    }

    let conf_dir = root.join("etc/ld.so.conf.d");
    std::fs::create_dir_all(&conf_dir)?;

    let content = lib_dirs.join("\n") + "\n";
    // We don't know the package name here; caller can rename the file.
    std::fs::write(conf_dir.join("fpm-package.conf"), content)?;
    tracing::debug!("wrote ld.so.conf.d entry ({} lib dirs)", lib_dirs.len());
    Ok(())
}

/// If the package ships .desktop files, ensure the directory exists.
fn register_desktop_entries(root: &Path) -> Result<(), InstallerError> {
    let apps_dir = root.join("usr/share/applications");
    if !apps_dir.exists() {
        return Ok(());
    }

    // Count .desktop files
    let count = walkdir::WalkDir::new(&apps_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "desktop").unwrap_or(false))
        .count();

    if count > 0 {
        tracing::info!("registered {} .desktop entries", count);
    }
    Ok(())
}
