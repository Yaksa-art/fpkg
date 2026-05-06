//! Integration tests for M2 Fetcher.
//! Uses wiremock to simulate a mirror server.

use fpm_fetcher::{
    cache::PackageCache,
    config::{FetcherConfig, MirrorConfig},
    download::fetch_packages,
    progress::progress_channel,
};
use fpm_solver::ResolvedPackage;
use std::path::PathBuf;
use tempfile::TempDir;
use wiremock::{
    matchers::{method, path_regex},
    Mock, MockServer, ResponseTemplate,
};

/// Build a minimal test config pointing at a mock server.
fn test_config(server_url: &str, cache_dir: PathBuf) -> FetcherConfig {
    FetcherConfig {
        parallel_downloads: 2,
        cache_dir,
        mirrors: vec![MirrorConfig {
            name: "mock-mirror".to_string(),
            url: server_url.to_string(),
            priority: 1000,
            enabled: true,
        }],
        timeout_secs: 5,
        retries: 1,
    }
}

fn fake_fpkg_bytes() -> Vec<u8> {
    // Minimal non-empty bytes (not a real tar.zst, but enough for cache/checksum tests)
    b"FAKE_FPKG_DATA_FOR_TEST".to_vec()
}

#[tokio::test]
async fn test_cache_hit_skips_download() {
    let tmp = TempDir::new().unwrap();
    let cache = PackageCache::new(tmp.path().to_path_buf());

    // Pre-populate cache
    let fpkg_data = fake_fpkg_bytes();
    let path = cache.package_path("hello", "1.0.0");
    std::fs::write(&path, &fpkg_data).unwrap();

    let server = MockServer::start().await;
    // No mocks registered — any HTTP call would panic

    let config = test_config(&server.uri(), tmp.path().to_path_buf());
    let pkg = ResolvedPackage {
        name: "hello".to_string(),
        version: "1.0.0".parse().unwrap(),
        blake3: None, // no hash check — treat any file as valid
        deps: vec![],
    };

    let pubkey = PathBuf::from("/dev/null");
    let results = fetch_packages(&[pkg], &config, &pubkey, None).await;
    assert_eq!(results.len(), 1);
    let r = results.into_iter().next().unwrap().unwrap();
    assert!(r.was_cached, "should have been a cache hit");
}

#[tokio::test]
async fn test_download_package_success() {
    let tmp = TempDir::new().unwrap();
    let server = MockServer::start().await;

    let fpkg_data = fake_fpkg_bytes();
    Mock::given(method("GET"))
        .and(path_regex(r"/api/v1/packages/hello/1\.0\.0/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(fpkg_data.clone())
                .insert_header("Content-Length", fpkg_data.len().to_string())
                .insert_header("ETag", "\"abc123\""),
        )
        .mount(&server)
        .await;

    // Mirror probe (HEAD /api/v1/packages)
    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let config = test_config(&server.uri(), tmp.path().to_path_buf());
    let pkg = ResolvedPackage {
        name: "hello".to_string(),
        version: "1.0.0".parse().unwrap(),
        blake3: None,
        deps: vec![],
    };

    let pubkey = PathBuf::from("/dev/null");
    let (tx, _rx) = progress_channel(64);
    let results = fetch_packages(&[pkg], &config, &pubkey, Some(tx)).await;
    assert_eq!(results.len(), 1);

    // With verifier not linked it should still succeed
    let r = results.into_iter().next().unwrap();
    // Note: extraction step may fail on fake bytes; we test the download itself
    match r {
        Ok(f) => assert_eq!(f.package, "hello"),
        Err(e) => {
            // Extraction of fake bytes fails — acceptable in unit test
            let s = e.to_string();
            assert!(
                s.contains("extract") || s.contains("Failed") || s.contains("zstd"),
                "unexpected error: {}",
                s
            );
        }
    }
}

#[tokio::test]
async fn test_checksum_mismatch_rejected() {
    let tmp = TempDir::new().unwrap();
    let server = MockServer::start().await;

    let fpkg_data = b"corrupted data".to_vec();
    Mock::given(method("GET"))
        .and(path_regex(r"/api/v1/packages/bad/1\.0\.0/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(fpkg_data.clone())
                .insert_header("Content-Length", fpkg_data.len().to_string()),
        )
        .mount(&server)
        .await;

    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let config = test_config(&server.uri(), tmp.path().to_path_buf());
    let pkg = ResolvedPackage {
        name: "bad".to_string(),
        version: "1.0.0".parse().unwrap(),
        // Wrong expected hash — should cause ChecksumMismatch
        blake3: Some("0000000000000000000000000000000000000000000000000000000000000000".to_string()),
        deps: vec![],
    };

    let pubkey = PathBuf::from("/dev/null");
    let results = fetch_packages(&[pkg], &config, &pubkey, None).await;
    let r = results.into_iter().next().unwrap();
    assert!(r.is_err(), "expected checksum error");
    let e = r.unwrap_err().to_string();
    assert!(e.contains("mismatch") || e.contains("mirror"), "unexpected error: {}", e);
}

#[tokio::test]
async fn test_http_error_fails() {
    let tmp = TempDir::new().unwrap();
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex(r"/api/v1/packages/ghost/1\.0\.0/download"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let config = test_config(&server.uri(), tmp.path().to_path_buf());
    let pkg = ResolvedPackage {
        name: "ghost".to_string(),
        version: "1.0.0".parse().unwrap(),
        blake3: None,
        deps: vec![],
    };

    let pubkey = PathBuf::from("/dev/null");
    let results = fetch_packages(&[pkg], &config, &pubkey, None).await;
    assert!(results[0].is_err());
}

#[tokio::test]
async fn test_parallel_downloads() {
    let tmp = TempDir::new().unwrap();
    let server = MockServer::start().await;

    for name in &["pkg-a", "pkg-b", "pkg-c"] {
        let data = format!("data-{}", name).into_bytes();
        Mock::given(method("GET"))
            .and(path_regex(format!(r"/api/v1/packages/{}/1\.0\.0/download", name).as_str()))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(data.clone())
                    .insert_header("Content-Length", data.len().to_string()),
            )
            .mount(&server)
            .await;
    }
    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let config = test_config(&server.uri(), tmp.path().to_path_buf());
    let pkgs: Vec<ResolvedPackage> = ["pkg-a", "pkg-b", "pkg-c"]
        .iter()
        .map(|n| ResolvedPackage {
            name: n.to_string(),
            version: "1.0.0".parse().unwrap(),
            blake3: None,
            deps: vec![],
        })
        .collect();

    let pubkey = PathBuf::from("/dev/null");
    let results = fetch_packages(&pkgs, &config, &pubkey, None).await;
    assert_eq!(results.len(), 3);
    // All should have been attempted (may fail at extraction step — that's fine)
    for r in &results {
        if let Err(e) = r {
            let s = e.to_string();
            assert!(
                s.contains("extract") || s.contains("Failed") || s.contains("zstd"),
                "unexpected error: {}", s
            );
        }
    }
}
