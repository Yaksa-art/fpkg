use std::path::{Path, PathBuf};
use crate::{
    error::IndexError,
    proto::RepoIndex,
};

pub struct IndexStore {
    base_dir: PathBuf,
}

impl IndexStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self { base_dir: base_dir.into() }
    }

    pub fn system() -> Self {
        Self::new("/var/lib/fpm/index")
    }

    fn path_for(&self, repo_name: &str) -> PathBuf {
        self.base_dir.join(format!("{}.msgpack", repo_name))
    }

    pub fn save(&self, repo_name: &str, index: &RepoIndex) -> Result<(), IndexError> {
        std::fs::create_dir_all(&self.base_dir)?;
        let bytes = rmp_serde::to_vec(index)?;
        let path = self.path_for(repo_name);
        std::fs::write(&path, &bytes)?;
        tracing::debug!("index: saved {} ({} bytes) → {:?}", repo_name, bytes.len(), path);
        Ok(())
    }

    pub fn load(&self, repo_name: &str) -> Result<Option<RepoIndex>, IndexError> {
        let path = self.path_for(repo_name);
        if !path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&path)?;
        let index: RepoIndex = rmp_serde::from_slice(&bytes)?;
        Ok(Some(index))
    }

    pub fn remove(&self, repo_name: &str) -> Result<(), IndexError> {
        let path = self.path_for(repo_name);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    pub fn exists(&self, repo_name: &str) -> bool {
        self.path_for(repo_name).exists()
    }

    pub fn list(&self) -> Result<Vec<String>, IndexError> {
        if !self.base_dir.exists() {
            return Ok(vec![]);
        }
        let mut names = vec![];
        for entry in std::fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            if let Some(s) = name.to_str() {
                if let Some(repo) = s.strip_suffix(".msgpack") {
                    names.push(repo.to_string());
                }
            }
        }
        Ok(names)
    }
}
