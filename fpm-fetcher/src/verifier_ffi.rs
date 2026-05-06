//! FFI bindings to M3 Verifier (libfpm_verifier.a).
//!
//! When fpm-verifier is built (../fpm-verifier/build/libfpm_verifier.a exists),
//! build.rs sets cfg(feature="verifier_linked") and the real FFI is active.
//! Otherwise we fall back to a stub that always returns Ok (dev / CI without C++ toolchain).

use crate::error::FetchError;
use std::ffi::CString;
use std::path::Path;

#[cfg(feature = "verifier_linked")]
mod ffi {
    #[repr(C)]
    pub struct FpmVerifyResult {
        pub code: i32,
        pub message: [u8; 256],
    }

    extern "C" {
        pub fn fpm_verify_package(
            extracted_dir: *const i8,
            pubkey_path: *const i8,
        ) -> FpmVerifyResult;
    }
}

/// Call M3 Verifier on an extracted .fpkg directory.
/// `extracted_dir` must contain META/ and DATA/ subdirectories.
/// `pubkey_path` is the path to the repo/package Ed25519 public key.
pub fn verify_package(extracted_dir: &Path, pubkey_path: &Path) -> Result<(), FetchError> {
    #[cfg(feature = "verifier_linked")]
    {
        let dir_c = CString::new(extracted_dir.to_str().unwrap_or("")).unwrap();
        let key_c = CString::new(pubkey_path.to_str().unwrap_or("")).unwrap();

        let result = unsafe {
            ffi::fpm_verify_package(
                dir_c.as_ptr() as *const i8,
                key_c.as_ptr() as *const i8,
            )
        };

        if result.code != 0 {
            let msg = result.message
                .iter()
                .take_while(|&&b| b != 0)
                .map(|&b| b as char)
                .collect::<String>();

            return Err(FetchError::VerificationFailed {
                package: extracted_dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                reason: msg,
            });
        }
        Ok(())
    }

    #[cfg(not(feature = "verifier_linked"))]
    {
        tracing::warn!(
            "M3 Verifier not linked — skipping cryptographic verification for {:?}",
            extracted_dir
        );
        Ok(())
    }
}
