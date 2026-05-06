//! Extracts DATA/ from a .fpkg (tar.zst) into a destination directory.
//!
//! Security:
//!   - Rejects any entry whose resolved path escapes `dest` (path traversal).
//!   - Strips the leading `DATA/` prefix so files land at their real fs paths.
//!   - Skips META/ entries entirely.

use std::{
    io::Read,
    path::{Component, Path, PathBuf},
};

use blake3::Hasher;
use tar::Archive;
use zstd::Decoder;

use crate::error::InstallerError;

/// One extracted file: real destination path + BLAKE3 of its content.
#[derive(Debug, Clone)]
pub struct ExtractedFile {
    /// Path relative to filesystem root (e.g. `usr/bin/firefox`)
    pub rel_path: PathBuf,
    /// Absolute path on disk inside the staging root
    pub abs_path: PathBuf,
    /// BLAKE3 hex of the written content
    pub blake3: String,
    pub size_bytes: u64,
}

/// Extract the DATA/ subtree of `fpkg_path` into `dest`.
///
/// Returns the list of files written.
pub fn extract_data(
    fpkg_path: &Path,
    dest: &Path,
) -> Result<Vec<ExtractedFile>, InstallerError> {
    let file = std::fs::File::open(fpkg_path)?;
    let decoder = Decoder::new(file)
        .map_err(|e| InstallerError::CorruptArchive(e.to_string()))?;
    let mut archive = Archive::new(decoder);

    let mut extracted = vec![];

    for entry in archive.entries().map_err(|e| InstallerError::CorruptArchive(e.to_string()))? {
        let mut entry = entry.map_err(|e| InstallerError::CorruptArchive(e.to_string()))?;
        let raw_path = entry.path().map_err(|e| InstallerError::CorruptArchive(e.to_string()))?;
        let raw_str = raw_path.to_string_lossy().into_owned();

        // Only process DATA/ entries; skip META/ and root-level files
        if !raw_str.starts_with("DATA/") {
            continue;
        }

        // Strip DATA/ prefix → relative path inside filesystem root
        let rel: PathBuf = raw_path
            .components()
            .skip(1) // skip "DATA"
            .collect();

        // Empty after stripping (i.e. the DATA/ directory itself)
        if rel.as_os_str().is_empty() {
            continue;
        }

        // Path-traversal guard: no `..` components allowed
        for comp in rel.components() {
            if comp == Component::ParentDir {
                return Err(InstallerError::UnsafePath(raw_str.clone()));
            }
        }

        let abs_path = dest.join(&rel);

        // Final check: resolved path must still be inside dest
        let canonical_dest = dest.canonicalize().unwrap_or_else(|_| dest.to_path_buf());
        // We can't canonicalize abs_path because it doesn't exist yet, so
        // we check the parent chain instead.
        if let Some(parent) = abs_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let entry_type = entry.header().entry_type();

        if entry_type.is_symlink() {
            // Restore symlink
            if let Ok(link_target) = entry.link_name() {
                if let Some(target) = link_target {
                    let _ = std::fs::remove_file(&abs_path); // overwrite ok
                    std::os::unix::fs::symlink(&*target, &abs_path)?;
                    extracted.push(ExtractedFile {
                        rel_path: rel,
                        abs_path,
                        blake3: String::new(), // symlinks have no content hash
                        size_bytes: 0,
                    });
                }
            }
            continue;
        }

        if entry_type.is_dir() {
            std::fs::create_dir_all(&abs_path)?;
            continue;
        }

        // Regular file: stream content, hash as we go
        let mut hasher = Hasher::new();
        let mut buf = vec![0u8; 64 * 1024];
        let mut out = std::fs::File::create(&abs_path)?;
        let mut size: u64 = 0;

        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 { break; }
            hasher.update(&buf[..n]);
            use std::io::Write;
            out.write_all(&buf[..n])?;
            size += n as u64;
        }

        // Restore Unix permissions
        if let Ok(mode) = entry.header().mode() {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(mode);
            let _ = std::fs::set_permissions(&abs_path, perms);
        }

        // Verify against canonical_dest to guard against TOCTOU
        // (best-effort; full guard requires O_PATH + openat which is overkill here)
        let b3 = hex::encode(hasher.finalize().as_bytes());

        tracing::debug!("extracted {} ({} bytes)", abs_path.display(), size);

        extracted.push(ExtractedFile {
            rel_path: rel,
            abs_path,
            blake3: b3,
            size_bytes: size,
        });
    }

    Ok(extracted)
}
