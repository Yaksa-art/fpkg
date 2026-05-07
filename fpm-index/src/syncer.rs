use reqwest::{
    header::{ETAG, IF_NONE_MATCH},
    StatusCode,
};
use fpm_db::{
    pool::DbPool,
    repos::RepoStore,
};
use fpm_solver::index::PackageIndex;

use crate::{
    delta,
    error::IndexError,
    loader,
    proto::{RepoDelta, RepoIndex},
    store::IndexStore,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncOutcome {
    Updated,
    NotModified,
    Created,
}

pub struct IndexSyncer {
    client: reqwest::Client,
    store: IndexStore,
}

impl IndexSyncer {
    pub fn new(store: IndexStore) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!("fpm-index/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client init");
        Self { client, store }
    }

    pub fn system() -> Self {
        Self::new(IndexStore::system())
    }

    pub async fn sync_repo(
        &self,
        name: &str,
        base_url: &str,
        etag: Option<&str>,
        pool: &DbPool,
    ) -> Result<SyncOutcome, IndexError> {
        let existing = self.store.load(name)?;

        if let (Some(ref base), Some(current_etag)) = (&existing, etag) {
            if let Some(outcome) = self.try_delta(name, base_url, base, current_etag, pool).await? {
                return Ok(outcome);
            }
        }

        self.fetch_full(name, base_url, etag, pool).await
    }

    async fn try_delta(
        &self,
        name: &str,
        base_url: &str,
        base: &RepoIndex,
        current_etag: &str,
        pool: &DbPool,
    ) -> Result<Option<SyncOutcome>, IndexError> {
        let delta_url = format!("{}/index.delta.msgpack", base_url.trim_end_matches('/'));
        let resp = self
            .client
            .get(&delta_url)
            .header(IF_NONE_MATCH, current_etag)
            .send()
            .await
            .map_err(|e| IndexError::Http { url: delta_url.clone(), source: e })?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if resp.status() == StatusCode::NOT_MODIFIED {
            return Ok(Some(SyncOutcome::NotModified));
        }

        if !resp.status().is_success() {
            return Ok(None);
        }

        let new_etag = resp
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| IndexError::Http { url: delta_url, source: e })?;

        let delta: RepoDelta = rmp_serde::from_slice(&bytes)?;

        if delta.base_etag != current_etag {
            return Ok(None);
        }

        let mut updated = base.clone();
        delta::apply(&mut updated, delta)?;
        self.store.save(name, &updated)?;

        let repos = RepoStore::new(pool);
        repos.mark_synced(name, new_etag.as_deref())?;

        tracing::info!("index: delta sync {} — {} packages", name, updated.packages.len());
        Ok(Some(SyncOutcome::Updated))
    }

    async fn fetch_full(
        &self,
        name: &str,
        base_url: &str,
        etag: Option<&str>,
        pool: &DbPool,
    ) -> Result<SyncOutcome, IndexError> {
        let index_url = format!("{}/index.msgpack", base_url.trim_end_matches('/'));

        let mut req = self.client.get(&index_url);
        if let Some(tag) = etag {
            req = req.header(IF_NONE_MATCH, tag);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| IndexError::Http { url: index_url.clone(), source: e })?;

        if resp.status() == StatusCode::NOT_MODIFIED {
            tracing::debug!("index: {} not modified (304)", name);
            return Ok(SyncOutcome::NotModified);
        }

        if !resp.status().is_success() {
            return Err(IndexError::Other(format!(
                "unexpected status {} for {}",
                resp.status(),
                index_url
            )));
        }

        let was_new = !self.store.exists(name);

        let new_etag = resp
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| IndexError::Http { url: index_url, source: e })?;

        let index: RepoIndex = rmp_serde::from_slice(&bytes)?;
        self.store.save(name, &index)?;

        let repos = RepoStore::new(pool);
        repos.mark_synced(name, new_etag.as_deref())?;

        tracing::info!("index: full sync {} — {} packages", name, index.packages.len());
        Ok(if was_new { SyncOutcome::Created } else { SyncOutcome::Updated })
    }

    pub async fn sync_all(&self, pool: &DbPool) -> Vec<(String, Result<SyncOutcome, IndexError>)> {
        let repos = RepoStore::new(pool);
        let all = match repos.list_enabled() {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("index: cannot read repo list: {}", e);
                return vec![];
            }
        };

        let mut results = vec![];
        for repo in all {
            let outcome = self
                .sync_repo(&repo.name, &repo.url, repo.etag.as_deref(), pool)
                .await;
            results.push((repo.name, outcome));
        }
        results
    }

    pub fn load_index(&self, repo_name: &str) -> Result<Option<PackageIndex>, IndexError> {
        match self.store.load(repo_name)? {
            Some(ri) => Ok(Some(loader::into_package_index(&ri))),
            None => Ok(None),
        }
    }

    pub fn load_merged_index(&self, pool: &DbPool) -> Result<PackageIndex, IndexError> {
        let repos = RepoStore::new(pool);
        let all = repos.list_enabled()?;
        let mut merged = PackageIndex::new();
        for repo in all {
            if let Some(ri) = self.store.load(&repo.name)? {
                let idx = loader::into_package_index(&ri);
                for name in ri.packages.iter().map(|p| &p.name) {
                    for ver in idx.versions_of(name) {
                        if let Some(rec) = idx.get(name, &ver) {
                            merged.add(rec.clone());
                        }
                    }
                }
            }
        }
        Ok(merged)
    }
}
