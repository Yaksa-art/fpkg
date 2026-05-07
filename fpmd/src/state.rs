use std::sync::Mutex;
use anyhow::Result;
use crate::config::DaemonConfig;

pub struct DaemonState {
    pub config: DaemonConfig,
    pub db:     Mutex<fpm_db::db::PackageDb>,
}

impl DaemonState {
    pub fn new(cfg: DaemonConfig) -> Result<Self> {
        if let Some(parent) = cfg.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = fpm_db::db::PackageDb::open(&cfg.db_path)?;
        Ok(Self {
            config: cfg,
            db: Mutex::new(db),
        })
    }
}
