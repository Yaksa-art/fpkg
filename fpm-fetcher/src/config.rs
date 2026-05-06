use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Loaded from /etc/fpm/fpm.conf or ~/.config/fpm/fpm.conf
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetcherConfig {
    /// Maximum parallel downloads
    #[serde(default = "default_parallel")]
    pub parallel_downloads: usize,

    /// Package cache directory
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,

    /// List of configured mirrors/repos
    pub mirrors: Vec<MirrorConfig>,

    /// Timeout per request in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Retry count per mirror before failover
    #[serde(default = "default_retries")]
    pub retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    pub name: String,
    pub url: String,
    pub priority: u32,
    pub enabled: bool,
}

fn default_parallel() -> usize { 4 }
fn default_cache_dir() -> PathBuf { PathBuf::from("/var/cache/fpm") }
fn default_timeout() -> u64 { 30 }
fn default_retries() -> u32 { 3 }

impl FetcherConfig {
    pub fn load_system() -> anyhow::Result<Self> {
        let path = std::path::Path::new("/etc/fpm/fpm.conf");
        Self::load_from(path)
    }

    pub fn load_user() -> anyhow::Result<Self> {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
        path.push("fpm/fpm.conf");
        Self::load_from(&path)
    }

    pub fn load_from(path: &std::path::Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&text)?;
        Ok(cfg)
    }

    /// Fallback config used in tests / when no config file exists
    pub fn default_system() -> Self {
        Self {
            parallel_downloads: 4,
            cache_dir: PathBuf::from("/var/cache/fpm"),
            mirrors: vec![],
            timeout_secs: 30,
            retries: 3,
        }
    }
}
