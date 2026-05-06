#[cfg(test)]
mod tests {
    use crate::{
        index::{PackageIndex, PackageRecord},
        solver::resolve,
        types::{Dep, Version, VersionReq},
    };

    fn ver(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn req(s: &str) -> VersionReq {
        VersionReq::parse(s)
    }

    fn simple_record(name: &str, version: &str, deps: Vec<Dep>) -> PackageRecord {
        PackageRecord {
            name: name.to_string(),
            version: ver(version),
            deps,
            provides: vec![],
            conflicts: vec![],
        }
    }

    fn build_simple_index() -> PackageIndex {
        let mut idx = PackageIndex::new();

        idx.add(PackageRecord {
            name: "glibc".into(),
            version: ver("2.38.0"),
            deps: vec![],
            provides: vec!["libc".into()],
            conflicts: vec![],
        });
        idx.add(PackageRecord {
            name: "glibc".into(),
            version: ver("2.35.0"),
            deps: vec![],
            provides: vec!["libc".into()],
            conflicts: vec![],
        });
        idx.add(simple_record("libdbus", "1.14.0", vec![
            Dep::required("glibc", req(">= 2.35")),
        ]));
        idx.add(simple_record("gtk3", "3.24.0", vec![
            Dep::required("glibc", req(">= 2.35")),
            Dep::required("libdbus", req(">= 1.12")),
        ]));

        idx
    }

    #[test]
    fn test_simple_dep_resolution() {
        let mut idx = build_simple_index();
        idx.add(simple_record("firefox", "125.0.0", vec![
            Dep::required("gtk3", req(">= 3.24")),
            Dep::required("glibc", req(">= 2.35")),
        ]));

        let res = resolve(&idx, "firefox", &ver("125.0.0")).unwrap();
        assert!(res.get("gtk3").is_some());
        assert!(res.get("glibc").is_some());
        assert!(res.get("libdbus").is_some());
    }

    #[test]
    fn test_transitive_deps() {
        let mut idx = PackageIndex::new();
        idx.add(simple_record("a", "1.0.0", vec![Dep::required("b", req(">= 1.0"))]));
        idx.add(simple_record("b", "1.0.0", vec![Dep::required("c", req(">= 1.0"))]));
        idx.add(simple_record("c", "1.0.0", vec![]));

        let res = resolve(&idx, "a", &ver("1.0.0")).unwrap();
        assert!(res.get("b").is_some());
        assert!(res.get("c").is_some());
    }

    #[test]
    fn test_version_conflict_detection() {
        let mut idx = PackageIndex::new();
        idx.add(simple_record("app", "1.0.0", vec![
            Dep::required("lib", req(">= 2.0")),
        ]));
        idx.add(simple_record("lib", "1.9.0", vec![]));

        let result = resolve(&idx, "app", &ver("1.0.0"));
        assert!(result.is_err(), "should fail: lib 2.0 not available");
    }

    #[test]
    fn test_provides_virtual_package() {
        let mut idx = PackageIndex::new();
        idx.add(PackageRecord {
            name: "musl".into(),
            version: ver("1.2.0"),
            deps: vec![],
            provides: vec!["libc".into()],
            conflicts: vec![],
        });
        idx.add(simple_record("busybox", "1.36.0", vec![
            Dep::required("libc", req(">= 1.0")),
        ]));

        let res = resolve(&idx, "busybox", &ver("1.36.0")).unwrap();
        assert!(res.get("musl").is_some());
    }

    #[test]
    fn test_optional_deps_excluded() {
        let mut idx = PackageIndex::new();
        idx.add(PackageRecord {
            name: "htop".into(),
            version: ver("3.3.0"),
            deps: vec![
                Dep { name: "glibc".into(), req: req(">= 2.17"), optional: false, reason: None },
                Dep { name: "lm-sensors".into(), req: req(">= 3.0"), optional: true, reason: Some("cpu temp".into()) },
            ],
            provides: vec![],
            conflicts: vec![],
        });
        idx.add(simple_record("glibc", "2.38.0", vec![]));

        let res = resolve(&idx, "htop", &ver("3.3.0")).unwrap();
        assert!(res.get("glibc").is_some());
        assert!(res.get("lm-sensors").is_none());
    }

    #[test]
    fn test_conflict_error_message() {
        let mut idx = PackageIndex::new();
        idx.add(PackageRecord {
            name: "firefox".into(),
            version: ver("125.0.0"),
            deps: vec![],
            provides: vec![],
            conflicts: vec!["firefox-esr".into()],
        });
        idx.add(simple_record("firefox-esr", "115.0.0", vec![]));
        idx.add(simple_record("meta-browser", "1.0.0", vec![
            Dep::required("firefox", req(">= 125.0")),
            Dep::required("firefox-esr", req(">= 115.0")),
        ]));

        let result = resolve(&idx, "meta-browser", &ver("1.0.0"));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("conflict") || msg.contains("firefox"));
    }
}
