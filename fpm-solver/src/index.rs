use std::collections::HashMap;
use crate::types::{Dep, Package, Version, VersionReq};

#[derive(Debug, Clone)]
pub struct PackageRecord {
    pub name: String,
    pub version: Version,
    pub deps: Vec<Dep>,
    pub provides: Vec<String>,
    pub conflicts: Vec<String>,
}

#[derive(Debug, Default)]
pub struct PackageIndex {
    packages: HashMap<String, Vec<PackageRecord>>,
    virtual_map: HashMap<String, Vec<String>>,
}

impl PackageIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, record: PackageRecord) {
        for provided in &record.provides {
            self.virtual_map
                .entry(provided.clone())
                .or_default()
                .push(record.name.clone());
        }
        self.packages
            .entry(record.name.clone())
            .or_default()
            .push(record);
    }

    pub fn versions_of(&self, name: &str) -> Vec<Version> {
        self.packages
            .get(name)
            .map(|recs| {
                let mut vs: Vec<Version> = recs.iter().map(|r| r.version.clone()).collect();
                vs.sort();
                vs.dedup();
                vs
            })
            .unwrap_or_default()
    }

    pub fn get(&self, name: &str, version: &Version) -> Option<&PackageRecord> {
        self.packages.get(name)?.iter().find(|r| &r.version == version)
    }

    pub fn providers_of(&self, virtual_name: &str) -> Vec<&str> {
        self.virtual_map
            .get(virtual_name)
            .map(|v| v.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    pub fn resolve_name<'a>(&'a self, name: &'a str) -> Vec<&'a str> {
        if self.packages.contains_key(name) {
            vec![name]
        } else {
            self.providers_of(name)
        }
    }

    pub fn satisfying_versions(&self, name: &str, req: &VersionReq) -> Vec<Version> {
        self.versions_of(name)
            .into_iter()
            .filter(|v| req.matches(v))
            .collect()
    }

    pub fn has_conflict(&self, pkg_name: &str, pkg_ver: &Version, candidate: &str) -> bool {
        if let Some(record) = self.get(pkg_name, pkg_ver) {
            record.conflicts.iter().any(|c| c == candidate)
        } else {
            false
        }
    }
}
