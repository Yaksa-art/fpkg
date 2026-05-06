use std::path::PathBuf;
use anyhow::{Context, Result};
use futures::StreamExt;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use crate::{
    cache::Cache,
    progress::Progress,
    types::{FetchError, PackageUrl},
};

#[derive(Debug, Clone)]
pub struct FetchRequest {
    pub package: PackageUrl,
    pub cache: Cache,
    pub parallel: usize,
    pub progress: Option<Progress>,
}

#[derive(Debug)]
pub struct FetchResult {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub from_cache: bool,
}

pub async fn fetch_all(requests: Vec<FetchRequest>) -> Vec<Result<FetchResult>> {
    let tasks: Vec<_> = requests.into_iter().map(|req| {
        tokio::spawn(async move { fetch_one(req).await })
    }).collect();

    futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|r| r.map_err(|e| anyhow::anyhow!("task panicked: {}", e)).and_then(|v| v))
        .collect()
}

pub async fn fetch_one(req: FetchRequest) -> Result<FetchResult> {
    let key = req.package.cache_key();

    req.cache.ensure_dir()?;

    if req.cache.contains(&key) {
        let path = req.cache.path_for(&key);
        if let Some(expected) = &req.package.blake3 {
            verify_file(&path, expected).await?;
        }
        return Ok(FetchResult {
            name: req.package.name.clone(),
            version: req.package.version.clone(),
            path,
            from_cache: true,
        });
    }

    let client = reqwest::Client::builder()
        .user_agent("fpm-fetcher/0.1")
        .use_rustls_tls()
        .build()
        .context("build http client")?;

    let mut last_err: Option<anyhow::Error> = None;

    for url in &req.package.urls {
        match download_with_resume(&client, url, &req.cache, &key, req.progress.clone()).await {
            Ok(()) => {
                if let Some(expected) = &req.package.blake3 {
                    let path = req.cache.path_for(&key);
                    if let Err(e) = verify_file(&path, expected).await {
                        req.cache.remove(&key)?;
                        return Err(e);
                    }
                }
                req.cache.commit_partial(&key)?;
                let path = req.cache.path_for(&key);
                return Ok(FetchResult {
                    name: req.package.name.clone(),
                    version: req.package.version.clone(),
                    path,
                    from_cache: false,
                });
            }
            Err(e) => {
                last_err = Some(e);
            }
        }
    }

    Err(last_err.unwrap_or_else(|| {
        FetchError::NoMirrors(req.package.name.clone()).into()
    }))
}

async fn download_with_resume(
    client: &reqwest::Client,
    url: &str,
    cache: &Cache,
    key: &str,
    progress: Option<Progress>,
) -> Result<()> {
    let partial_path = cache.partial_path_for(key);

    let existing_bytes = if partial_path.exists() {
        tokio::fs::metadata(&partial_path).await?.len()
    } else {
        0
    };

    let mut request = client.get(url);
    if existing_bytes > 0 {
        request = request.header("Range", format!("bytes={}-", existing_bytes));
    }

    let response = request.send().await.map_err(|e| FetchError::Download {
        url: url.to_string(),
        source: e,
    })?;

    let status = response.status();
    let is_partial = status == reqwest::StatusCode::PARTIAL_CONTENT;
    let is_ok = status.is_success();

    if !is_ok && !is_partial {
        return Err(anyhow::anyhow!("server returned {} for {}", status, url));
    }

    let file = if is_partial {
        let mut f = tokio::fs::OpenOptions::new()
            .append(true)
            .open(&partial_path)
            .await?;
        f.seek(std::io::SeekFrom::End(0)).await?;
        f
    } else {
        tokio::fs::File::create(&partial_path).await?
    };

    let mut writer = tokio::io::BufWriter::new(file);
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| FetchError::Download {
            url: url.to_string(),
            source: e,
        })?;
        if let Some(ref p) = progress {
            p.add(bytes.len() as u64);
        }
        writer.write_all(&bytes).await?;
    }
    writer.flush().await?;

    Ok(())
}

async fn verify_file(path: &std::path::Path, expected_hex: &str) -> Result<()> {
    let data = tokio::fs::read(path).await?;
    let actual = blake3::hash(&data);
    let actual_hex = actual.to_hex().to_string();
    if actual_hex != expected_hex {
        return Err(FetchError::HashMismatch {
            path: path.display().to_string(),
            expected: expected_hex.to_string(),
            actual: actual_hex,
        }.into());
    }
    Ok(())
}
