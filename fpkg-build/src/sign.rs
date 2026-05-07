use std::{path::Path, process::Command};
use tracing::info;
use crate::BuildError;

pub fn sign_fpkg(fpkg_path: &Path, key_path: Option<&Path>) -> Result<(), BuildError> {
    let signer = which::which("fpkg-sign")
        .map_err(|_| BuildError::Sign("fpkg-sign not found in PATH".into()))?;

    let mut cmd = Command::new(signer);
    cmd.arg(fpkg_path);

    if let Some(key) = key_path {
        cmd.args(["--key", key.to_str().unwrap()]);
    }

    info!(fpkg = %fpkg_path.display(), "signing .fpkg");

    let status = cmd.status()?;
    if !status.success() {
        return Err(BuildError::Sign(
            format!("fpkg-sign exited with {:?}", status.code())
        ));
    }
    Ok(())
}

pub fn verify_signature_present(fpkg_path: &Path) -> bool {
    use std::io::Read;
    let f = match std::fs::File::open(fpkg_path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut zip = match zip::ZipArchive::new(f) {
        Ok(z) => z,
        Err(_) => return false,
    };
    zip.by_name("META/signature.ed25519").is_ok()
}
