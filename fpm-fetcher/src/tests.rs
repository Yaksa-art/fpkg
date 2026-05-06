#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::{
        cache::Cache,
        fetcher::{fetch_one, FetchRequest},
        progress::Progress,
        types::PackageUrl,
    };

    fn tmp_cache() -> Cache {
        let dir = std::env::temp_dir().join(format!("fpm-fetcher-test-{}", std::process::id()));
        Cache::new(dir)
    }

    fn pkg(name: &str, urls: Vec<String>, blake3: Option<String>) -> PackageUrl {
        PackageUrl {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            urls,
            blake3,
            size: None,
        }
    }

    #[tokio::test]
    async fn test_cache_hit_skips_download() {
        let cache = tmp_cache();
        cache.ensure_dir().unwrap();
        let key = "hello-1.0.0";
        let dest = cache.path_for(key);
        std::fs::write(&dest, b"fake-package-data").unwrap();

        let p = PackageUrl {
            name: "hello".into(),
            version: "1.0.0".into(),
            urls: vec!["http://unreachable.invalid/hello.fpkg".into()],
            blake3: None,
            size: None,
        };
        let req = FetchRequest { package: p, cache, parallel: 1, progress: None };
        let result = fetch_one(req).await.unwrap();
        assert!(result.from_cache);
        assert_eq!(result.path, dest);
    }

    #[tokio::test]
    async fn test_hash_mismatch_rejected() {
        let cache = tmp_cache();
        cache.ensure_dir().unwrap();
        let key = "badpkg-1.0.0";
        let dest = cache.path_for(key);
        std::fs::write(&dest, b"wrong data").unwrap();

        let p = PackageUrl {
            name: "badpkg".into(),
            version: "1.0.0".into(),
            urls: vec![],
            blake3: Some("0000000000000000000000000000000000000000000000000000000000000000".into()),
            size: None,
        };
        let req = FetchRequest { package: p, cache, parallel: 1, progress: None };
        let result = fetch_one(req).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("hash mismatch") || msg.contains("mismatch"));
    }

    #[tokio::test]
    async fn test_no_urls_returns_error() {
        let cache = tmp_cache();
        cache.ensure_dir().unwrap();
        let p = pkg("ghost", vec![], None);
        let req = FetchRequest { package: p, cache, parallel: 1, progress: None };
        let result = fetch_one(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_progress_tracks_bytes() {
        let progress = crate::progress::Progress::new(1000);
        progress.add(300);
        progress.add(400);
        assert_eq!(progress.downloaded_bytes(), 700);
        assert!((progress.fraction() - 0.7).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_cache_key_format() {
        let p = PackageUrl {
            name: "firefox".into(),
            version: "125.0.3".into(),
            urls: vec![],
            blake3: None,
            size: None,
        };
        assert_eq!(p.cache_key(), "firefox-125.0.3");
    }
}
