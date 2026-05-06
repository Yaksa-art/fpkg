use std::collections::HashMap;
use anyhow::{bail, Result};
use pubgrub::{
    error::PubGrubError,
    package::Package as PubPackage,
    range::Range,
    report::{DefaultStringReporter, Reporter},
    solver::{choose_package_with_fewest_versions, resolve as pubgrub_resolve, DependencyProvider},
    type_aliases::Map,
    version::Version as PubVersion,
    version_set::VersionSet,
};
use crate::{
    index::PackageIndex,
    types::{Package, Version, VersionReq},
};

#[derive(Debug, Clone)]
pub struct Resolution {
    pub packages: HashMap<String, Version>,
}

impl Resolution {
    pub fn get(&self, name: &str) -> Option<&Version> {
        self.packages.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Version)> {
        self.packages.iter()
    }
}

pub fn resolve(
    index: &PackageIndex,
    root_name: &str,
    root_version: &Version,
) -> Result<Resolution> {
    let provider = IndexProvider { index };

    let root_pkg = SemVer(root_name.to_string());
    let root_ver = SemVerVersion(root_version.clone());

    let solution = pubgrub_resolve(&provider, root_pkg, root_ver)
        .map_err(|e| match e {
            PubGrubError::NoSolution(tree) => {
                let report = DefaultStringReporter::report(&tree);
                anyhow::anyhow!("dependency conflict:\n{}", report)
            }
            other => anyhow::anyhow!("solver error: {:?}", other),
        })?;

    let mut packages = HashMap::new();
    for (pkg, ver) in solution {
        if pkg.0 != root_name {
            packages.insert(pkg.0, ver.0);
        }
    }
    packages.insert(root_name.to_string(), root_version.clone());

    Ok(Resolution { packages })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SemVer(String);

impl PubPackage for SemVer {}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SemVerVersion(Version);

impl PubVersion for SemVerVersion {
    fn lowest() -> Self {
        SemVerVersion(Version::new(0, 0, 0))
    }

    fn bump(&self) -> Self {
        SemVerVersion(Version::new(self.0.major, self.0.minor, self.0.patch + 1))
    }
}

impl std::fmt::Display for SemVerVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

struct IndexProvider<'a> {
    index: &'a PackageIndex,
}

impl<'a> DependencyProvider<SemVer, SemVerVersion> for IndexProvider<'a> {
    fn choose_package_version<T: std::borrow::Borrow<SemVer>, U: std::borrow::Borrow<Range<SemVerVersion>>>(
        &self,
        potential_packages: impl Iterator<Item = (T, U)>,
    ) -> Result<(SemVer, Option<SemVerVersion>), Box<dyn std::error::Error>> {
        Ok(choose_package_with_fewest_versions(
            |pkg: &SemVer| self.index.versions_of(&pkg.0).into_iter().map(SemVerVersion),
            potential_packages,
        ))
    }

    fn get_dependencies(
        &self,
        package: &SemVer,
        version: &SemVerVersion,
    ) -> Result<pubgrub::solver::Dependencies<SemVer, SemVerVersion>, Box<dyn std::error::Error>> {
        use pubgrub::solver::Dependencies;

        let record = match self.index.get(&package.0, &version.0) {
            Some(r) => r,
            None => return Ok(Dependencies::Known(Map::default())),
        };

        let mut map: Map<SemVer, Range<SemVerVersion>> = Map::default();

        for dep in record.deps.iter().filter(|d| !d.optional) {
            let resolved_names = self.index.resolve_name(&dep.name);
            if resolved_names.is_empty() {
                return Err(anyhow::anyhow!("unknown package: {}", dep.name).into());
            }
            let provider_name = resolved_names[0].to_string();

            let range = req_to_range(&dep.req);
            map.insert(SemVer(provider_name), range);
        }

        for conflict in &record.conflicts {
            map.insert(SemVer(conflict.clone()), Range::empty());
        }

        Ok(Dependencies::Known(map))
    }
}

fn req_to_range(req: &VersionReq) -> Range<SemVerVersion> {
    use crate::types::Op;
    let v = SemVerVersion(req.version.clone());
    match req.op {
        Op::Any => Range::full(),
        Op::Gte => Range::higher_than(v),
        Op::Gt  => Range::higher_than(v.bump()),
        Op::Lte => Range::strictly_lower_than(v.bump()),
        Op::Lt  => Range::strictly_lower_than(v),
        Op::Eq  => Range::singleton(v),
    }
}
