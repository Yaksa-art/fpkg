use std::path::{Path, PathBuf};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn system() -> Self {
        Self::new("/var/cache/fpm")
    }

    pub fn user() -> Self {
        let base = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        Self::new(format!("{}/.cache/fpm", base))
    }

    pub fn from_env(user_mode: bool) -> Self {
        if let Ok(p) = std::env::var("FPM_CACHE") {
            return Self::new(p);
        }
        if user_mode { Self::user() } else { Self::system() }
    }

    pub fn path_for(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.fpkg", key))
    }

    pub fn partial_path_for(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.fpkg.part", key))
    }

    pub fn contains(&self, key: &str) -> bool {
        self.path_for(key).exists()
    }

    pub fn ensure_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }

    pub fn remove(&self, key: &str) -> Result<()> {
        let p = self.path_for(key);
        if p.exists() {
            std::fs::remove_file(p)?;
        }
        Ok(())
    }

    pub fn commit_partial(&self, key: &str) -> Result<()> {
        let partial = self.partial_path_for(key);
        let final_path = self.path_for(key);
        std::fs::rename(partial, final_path)?;
        Ok(())
    }
}
