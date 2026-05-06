use std::time::{Duration, Instant};
use anyhow::Result;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Mirror {
    pub url: String,
    pub priority: u32,
}

impl Mirror {
    pub fn new(url: impl Into<String>, priority: u32) -> Self {
        Self { url: url.into(), priority }
    }
}

pub async fn probe_mirrors(mirrors: &[Mirror], timeout: Duration) -> Vec<Mirror> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .unwrap_or_default();

    let mut results: Vec<(Duration, Mirror)> = futures::future::join_all(
        mirrors.iter().map(|m| {
            let client = client.clone();
            let mirror = m.clone();
            async move {
                let start = Instant::now();
                let probe_url = format!("{}/ping", mirror.url.trim_end_matches('/'));
                let ok = client.head(&probe_url).send().await.is_ok();
                (if ok { start.elapsed() } else { Duration::MAX }, mirror)
            }
        })
    ).await;

    results.retain(|(d, _)| *d != Duration::MAX);
    results.sort_by_key(|(d, _)| *d);
    results.into_iter().map(|(_, m)| m).collect()
}
