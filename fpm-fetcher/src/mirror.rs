use crate::config::MirrorConfig;
use serde::{Deserialize, Serialize};

/// A resolved mirror with measured latency (ms). Lower = better.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mirror {
    pub name: String,
    pub base_url: String,
    pub priority: u32,
    pub latency_ms: u64,
}

impl Mirror {
    pub fn from_config(cfg: &MirrorConfig) -> Self {
        Self {
            name: cfg.name.clone(),
            base_url: cfg.url.trim_end_matches('/').to_string(),
            priority: cfg.priority,
            latency_ms: 0,
        }
    }

    /// URL for downloading a specific package version
    pub fn package_url(&self, name: &str, version: &str) -> String {
        format!("{}/api/v1/packages/{}/{}/download", self.base_url, name, version)
    }

    /// URL for the package signature
    pub fn signature_url(&self, name: &str, version: &str) -> String {
        format!("{}/api/v1/packages/{}/{}/signature", self.base_url, name, version)
    }

    /// URL for the repo public key
    pub fn pubkey_url(&self) -> String {
        format!("{}/api/v1/keys/repo.pub", self.base_url)
    }
}

/// Sort mirrors: lowest latency first, then highest priority.
pub fn rank_mirrors(mut mirrors: Vec<Mirror>) -> Vec<Mirror> {
    mirrors.sort_by(|a, b| {
        a.latency_ms
            .cmp(&b.latency_ms)
            .then(b.priority.cmp(&a.priority))
    });
    mirrors
}

/// Ping each mirror with a HEAD request and record latency.
pub async fn probe_mirrors(mirrors: Vec<Mirror>, client: &reqwest::Client) -> Vec<Mirror> {
    let mut results = Vec::with_capacity(mirrors.len());
    for mut m in mirrors {
        let url = format!("{}/api/v1/packages", m.base_url);
        let start = std::time::Instant::now();
        match client.head(&url).send().await {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 404 => {
                m.latency_ms = start.elapsed().as_millis() as u64;
                results.push(m);
            }
            _ => {
                tracing::warn!("Mirror {} unreachable, skipping", m.name);
            }
        }
    }
    rank_mirrors(results)
}
