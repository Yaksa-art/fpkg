use fpm_solver::{
    index::{PackageIndex, PackageRecord},
    types::{Dep, Version, VersionReq},
};
use crate::proto::RepoIndex;

pub fn into_package_index(repo: &RepoIndex) -> PackageIndex {
    let mut idx = PackageIndex::new();
    for pkg in &repo.packages {
        let version = Version::parse(&pkg.version).unwrap_or(Version::new(0, 0, 0));
        let deps = pkg
            .deps
            .iter()
            .map(|d| Dep {
                name: d.name.clone(),
                req: if d.version_req.is_empty() {
                    VersionReq::any()
                } else {
                    VersionReq::parse(&d.version_req)
                },
                optional: d.optional,
                reason: None,
            })
            .collect();
        idx.add(PackageRecord {
            name: pkg.name.clone(),
            version,
            deps,
            provides: pkg.provides.clone(),
            conflicts: pkg.conflicts.clone(),
        });
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{IndexDep, IndexPackage, RepoIndex};

    #[test]
    fn converts_repo_index_to_package_index() {
        let repo = RepoIndex {
            repo: "main".to_string(),
            generated_at: String::new(),
            packages: vec![
                IndexPackage {
                    name: "libfoo".to_string(),
                    version: "1.2.3".to_string(),
                    deps: vec![IndexDep {
                        name: "libbar".to_string(),
                        version_req: ">= 2.0.0".to_string(),
                        optional: false,
                    }],
                    provides: vec!["libfoo-compat".to_string()],
                    conflicts: vec![],
                    blake3: "abc".to_string(),
                    size: 1024,
                    url_path: "/pool/libfoo-1.2.3.fpkg".to_string(),
                },
            ],
        };

        let idx = into_package_index(&repo);
        let versions = idx.versions_of("libfoo");
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0], Version::new(1, 2, 3));

        let providers = idx.providers_of("libfoo-compat");
        assert_eq!(providers, vec!["libfoo"]);
    }
}
