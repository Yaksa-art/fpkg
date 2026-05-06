use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::StreamExt;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

use crate::{
    cache::PackageCache,
    config::FetcherConfig,
    error::FetchError,
    mirror::{probe_mirrors, Mirror},
    progress::{ProgressEvent, ProgressSender},
    verifier_ffi,
};
use fpm_solver::ResolvedPackage;

/// Result of fetching a single package.
#[derive(Debug, Clone)]
pub struct FetchResult {
    pub package: String,
    pub version: String,
    /// Absolute path to verified .fpkg in cache
    pub path: PathBuf,
    pub was_cached: bool,
}

/// Fetch all packages in the resolved list.
/// Downloads run in parallel (bounded by config.parallel_downloads).
/// Each package is verified by M3 after download.
///
/// `pubkey_path` — path to the repo Ed25519 public key used by M3 Verifier.
pub async fn fetch_packages(
    resolved: &[ResolvedPackage],
    config: &FetcherConfig,
    pubkey_path: &Path,
    progress: Option<ProgressSender>,
) -> Vec<Result<FetchResult, FetchError>> {
    let client = build_client(config);
    let mirrors = build_mirrors(config, &client).await;
    let cache = Arc::new(PackageCache::new(config.cache_dir.clone()));
    let sem = Arc::new(Semaphore::new(config.parallel_downloads));

    if let Err(e) = cache.ensure_dir() {
        tracing::error!("Cannot create cache dir: {}", e);
    }

    let mut handles = vec![];
    for pkg in resolved {
        let pkg = pkg.clone();
        let client = client.clone();
        let mirrors = mirrors.clone();
        let cache = cache.clone();
        let sem = sem.clone();
        let progress = progress.clone();
        let pubkey_path = pubkey_path.to_path_buf();
        let config = config.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            fetch_one(&pkg, &client, &mirrors, &cache, &pubkey_path, &config, progress).await
        });
        handles.push(handle);
    }

    let mut results = vec![];
    for h in handles {
        match h.await {
            Ok(r) => results.push(r),
            Err(e) => results.push(Err(FetchError::Cache(e.to_string()))),
        }
    }
    results
}

async fn fetch_one(
    pkg: &ResolvedPackage,
    client: &reqwest::Client,
    mirrors: &[Mirror],
    cache: &PackageCache,
    pubkey_path: &Path,
    config: &FetcherConfig,
    progress: Option<ProgressSender>,
) -> Result<FetchResult, FetchError> {
    let name = &pkg.name;
    let version = pkg.version.to_string();

    // Cache hit
    if cache.is_cached(name, &version, pkg.blake3.as_deref()) {
        let path = cache.package_path(name, &version);
        emit(&progress, ProgressEvent::Done {
            package: name.clone(),
            path: path.to_string_lossy().to_string(),
        }).await;
        return Ok(FetchResult {
            package: name.clone(),
            version: version.clone(),
            path,
            was_cached: true,
        });
    }

    // Try each mirror in order
    let mut last_err = FetchError::AllMirrorsFailed { package: name.clone() };
    for mirror in mirrors {
        match fetch_from_mirror(pkg, client, mirror, cache, pubkey_path, config, &progress).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                tracing::warn!("Mirror {} failed for {}: {}", mirror.name, name, e);
                emit(&progress, ProgressEvent::Error {
                    package: name.clone(),
                    reason: format!("Mirror {}: {}", mirror.name, e),
                }).await;
                last_err = e;
            }
        }
    }
    Err(last_err)
}

async fn fetch_from_mirror(
    pkg: &ResolvedPackage,
    client: &reqwest::Client,
    mirror: &Mirror,
    cache: &PackageCache,
    pubkey_path: &Path,
    config: &FetcherConfig,
    progress: &Option<ProgressSender>,
) -> Result<FetchResult, FetchError> {
    let name = &pkg.name;
    let version = pkg.version.to_string();
    let url = mirror.package_url(name, &version);

    // Resume support: check if .part file exists
    let partial_size = cache.partial_size(name, &version);
    let mut request = client.get(&url);
    if partial_size > 0 {
        request = request.header("Range", format!("bytes={}-", partial_size));
        tracing::info!("Resuming {} from byte {}", name, partial_size);
    }

    // ETag conditional
    if let Some(etag) = cache.read_etag(name, &version) {
        if partial_size == 0 {
            request = request.header("If-None-Match", etag);
        }
    }

    let resp = request.send().await?;
    let status = resp.status();

    // 304 Not Modified — cached copy is still valid
    if status.as_u16() == 304 {
        let path = cache.package_path(name, &version);
        return Ok(FetchResult {
            package: name.clone(),
            version,
            path,
            was_cached: true,
        });
    }

    if !status.is_success() {
        return Err(FetchError::Http {
            status: status.as_u16(),
            url: url.clone(),
        });
    }

    let total_bytes = resp.content_length().unwrap_or(0);
    emit(progress, ProgressEvent::Started {
        package: name.clone(),
        version: version.clone(),
        total_bytes: total_bytes + partial_size,
    }).await;

    // Save ETag for next time
    if let Some(etag) = resp.headers().get("ETag").and_then(|v| v.to_str().ok()) {
        cache.write_etag(name, &version, etag);
    }

    // Stream body → .part file
    let partial_path = cache.partial_path(name, &version);
    let mut file = OpenOptions::new()
        .create(true)
        .append(partial_size > 0)
        .write(true)
        .open(&partial_path)
        .await?;

    let mut received = partial_size;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        file.write_all(&bytes).await?;
        received += bytes.len() as u64;
        emit(progress, ProgressEvent::Chunk {
            package: name.clone(),
            received_bytes: received,
            total_bytes: total_bytes + partial_size,
        }).await;
    }
    file.flush().await?;
    drop(file);

    // Verify BLAKE3 if expected hash known
    if let Some(expected) = &pkg.blake3 {
        let data = tokio::fs::read(&partial_path).await?;
        let actual = hex::encode(blake3::hash(&data).as_bytes());
        if &actual != expected {
            let _ = tokio::fs::remove_file(&partial_path).await;
            return Err(FetchError::ChecksumMismatch {
                path: name.clone(),
                expected: expected.clone(),
                actual,
            });
        }
    }

    // Promote .part → final
    let final_path = cache.commit_partial(name, &version)?;

    emit(progress, ProgressEvent::Downloaded { package: name.clone() }).await;

    // === Call M3 Verifier ===
    // Extract to temp dir and verify Ed25519 + Merkle + checksums
    let extract_dir = extract_fpkg(&final_path).await?;
    match verifier_ffi::verify_package(&extract_dir, pubkey_path) {
        Ok(()) => {
            emit(progress, ProgressEvent::Verified {
                package: name.clone(),
                ok: true,
                reason: None,
            }).await;
        }
        Err(e) => {
            // Remove bad package from cache
            let _ = tokio::fs::remove_file(&final_path).await;
            emit(progress, ProgressEvent::Verified {
                package: name.clone(),
                ok: false,
                reason: Some(e.to_string()),
            }).await;
            return Err(e);
        }
    }

    emit(progress, ProgressEvent::Done {
        package: name.clone(),
        path: final_path.to_string_lossy().to_string(),
    }).await;

    Ok(FetchResult {
        package: name.clone(),
        version,
        path: final_path,
        was_cached: false,
    })
}

/// Extract a .fpkg (tar.zst) to a temp directory and return the path.
async fn extract_fpkg(fpkg_path: &Path) -> Result<PathBuf, FetchError> {
    let dest = std::env::temp_dir().join(format!(
        "fpm-verify-{}",
        uuid::Uuid::new_v4()
    ));
    tokio::fs::create_dir_all(&dest).await?;

    // Run `tar -I zstd -xf <fpkg> -C <dest>` asynchronously
    let status = tokio::process::Command::new("tar")
        .args(["-I", "zstd", "-xf"])
        .arg(fpkg_path)
        .args(["-C"])
        .arg(&dest)
        .status()
        .await
        .map_err(|e| FetchError::Io(e))?;

    if !status.success() {
        return Err(FetchError::Cache(format!(
            "Failed to extract {:?}",
            fpkg_path
        )));
    }

    Ok(dest)
}

async fn emit(tx: &Option<ProgressSender>, event: ProgressEvent) {
    if let Some(tx) = tx {
        let _ = tx.send(event).await;
    }
}

fn build_client(config: &FetcherConfig) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .user_agent("fpm/0.1.0 (FSocietyOS)")
        .build()
        .expect("Failed to build HTTP client")
}

async fn build_mirrors(config: &FetcherConfig, client: &reqwest::Client) -> Vec<Mirror> {
    let mirrors: Vec<Mirror> = config
        .mirrors
        .iter()
        .filter(|m| m.enabled)
        .map(Mirror::from_config)
        .collect();

    if mirrors.is_empty() {
        return mirrors;
    }

    probe_mirrors(mirrors, client).await
}
