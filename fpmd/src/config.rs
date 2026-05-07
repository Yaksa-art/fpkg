use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub mode:           String,
    pub socket_path:    PathBuf,
    pub db_path:        PathBuf,
    pub cache_dir:      PathBuf,
    pub log_level:      String,
    pub keep_generations: u32,
    pub parallel_downloads: u32,
    pub repos:          Vec<RepoConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub name:     String,
    pub url:      String,
    pub priority: u32,
    pub enabled:  bool,
    pub security: Option<bool>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            mode:               "system".into(),
            socket_path:        PathBuf::from("/run/fpm/fpmd.sock"),
            db_path:            PathBuf::from("/var/lib/fpm/db.sqlite"),
            cache_dir:          PathBuf::from("/var/cache/fpm"),
            log_level:          "info".into(),
            keep_generations:   5,
            parallel_downloads: 4,
            repos:              vec![],
        }
    }
}

pub fn load(path: Option<&Path>, mode: &str) -> Result<DaemonConfig> {
    let candidates: Vec<PathBuf> = if let Some(p) = path {
        vec![p.to_path_buf()]
    } else if mode == "user" {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        vec![
            PathBuf::from(&home).join(".config/fpm/fpm.conf"),
        ]
    } else {
        vec![
            PathBuf::from("/etc/fpm/fpm.conf"),
        ]
    };

    for candidate in &candidates {
        if candidate.exists() {
            debug!(path = %candidate.display(), "loading config");
            let raw = std::fs::read_to_string(candidate)?;
            let mut cfg: DaemonConfig = toml::from_str(&raw)
                .map_err(|e| anyhow::anyhow!("config parse error: {e}"))?;
            cfg.mode = mode.into();
            apply_mode_defaults(&mut cfg, mode);
            return Ok(cfg);
        }
    }

    let mut cfg = DaemonConfig::default();
    cfg.mode = mode.into();
    apply_mode_defaults(&mut cfg, mode);
    Ok(cfg)
}

fn apply_mode_defaults(cfg: &mut DaemonConfig, mode: &str) {
    if mode == "user" {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let uid  = nix_uid();
        if cfg.socket_path == PathBuf::from("/run/fpm/fpmd.sock") {
            cfg.socket_path = PathBuf::from(format!("/run/user/{uid}/fpm/fpmd.sock"));
        }
        if cfg.db_path == PathBuf::from("/var/lib/fpm/db.sqlite") {
            cfg.db_path = PathBuf::from(&home).join(".local/share/fpm/db.sqlite");
        }
        if cfg.cache_dir == PathBuf::from("/var/cache/fpm") {
            cfg.cache_dir = PathBuf::from(&home).join(".cache/fpm");
        }
    }
}

fn nix_uid() -> u32 {
    unsafe { libc::getuid() }
}
