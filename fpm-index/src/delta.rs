use crate::{
    error::IndexError,
    proto::{DeltaOp, RepoDelta, RepoIndex},
};

pub fn apply(base: &mut RepoIndex, delta: RepoDelta) -> Result<(), IndexError> {
    for entry in delta.entries {
        match entry.op {
            DeltaOp::Add => {
                base.packages.push(entry.package);
            }
            DeltaOp::Remove => {
                base.packages
                    .retain(|p| !(p.name == entry.package.name && p.version == entry.package.version));
            }
            DeltaOp::Update => {
                let pos = base
                    .packages
                    .iter()
                    .position(|p| p.name == entry.package.name && p.version == entry.package.version);
                match pos {
                    Some(i) => base.packages[i] = entry.package,
                    None => base.packages.push(entry.package),
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{DeltaEntry, IndexPackage, RepoDelta, RepoIndex};

    fn pkg(name: &str, version: &str) -> IndexPackage {
        IndexPackage {
            name: name.to_string(),
            version: version.to_string(),
            deps: vec![],
            provides: vec![],
            conflicts: vec![],
            blake3: "abc".to_string(),
            size: 0,
            url_path: String::new(),
        }
    }

    fn base() -> RepoIndex {
        RepoIndex {
            repo: "test".to_string(),
            generated_at: String::new(),
            packages: vec![pkg("libfoo", "1.0.0"), pkg("libbar", "2.0.0")],
        }
    }

    #[test]
    fn add_new_package() {
        let mut idx = base();
        let delta = RepoDelta {
            repo: "test".to_string(),
            base_etag: String::new(),
            entries: vec![DeltaEntry { op: DeltaOp::Add, package: pkg("newpkg", "0.1.0") }],
        };
        apply(&mut idx, delta).unwrap();
        assert_eq!(idx.packages.len(), 3);
        assert!(idx.packages.iter().any(|p| p.name == "newpkg"));
    }

    #[test]
    fn remove_package() {
        let mut idx = base();
        let delta = RepoDelta {
            repo: "test".to_string(),
            base_etag: String::new(),
            entries: vec![DeltaEntry { op: DeltaOp::Remove, package: pkg("libbar", "2.0.0") }],
        };
        apply(&mut idx, delta).unwrap();
        assert_eq!(idx.packages.len(), 1);
        assert!(!idx.packages.iter().any(|p| p.name == "libbar"));
    }

    #[test]
    fn update_package() {
        let mut idx = base();
        let mut updated = pkg("libfoo", "1.0.0");
        updated.blake3 = "new_hash".to_string();
        let delta = RepoDelta {
            repo: "test".to_string(),
            base_etag: String::new(),
            entries: vec![DeltaEntry { op: DeltaOp::Update, package: updated }],
        };
        apply(&mut idx, delta).unwrap();
        assert_eq!(idx.packages.len(), 2);
        let p = idx.packages.iter().find(|p| p.name == "libfoo").unwrap();
        assert_eq!(p.blake3, "new_hash");
    }
}
