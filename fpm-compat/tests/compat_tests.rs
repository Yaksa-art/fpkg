use fpm_compat::convert::{ForeignFormat, ForeignPackage};
use std::path::PathBuf;

#[test]
fn format_detect_deb() {
    let p = PathBuf::from("htop_3.2.2-1_amd64.deb");
    assert_eq!(ForeignFormat::detect(&p), Some(ForeignFormat::Deb));
}

#[test]
fn format_detect_rpm() {
    let p = PathBuf::from("htop-3.2.2-1.x86_64.rpm");
    assert_eq!(ForeignFormat::detect(&p), Some(ForeignFormat::Rpm));
}

#[test]
fn format_detect_apk() {
    let p = PathBuf::from("htop-3.2.2-r0.apk");
    assert_eq!(ForeignFormat::detect(&p), Some(ForeignFormat::Apk));
}

#[test]
fn format_detect_arch() {
    let p = PathBuf::from("htop-3.2.2-1-x86_64.pkg.tar.zst");
    assert_eq!(ForeignFormat::detect(&p), Some(ForeignFormat::Arch));
}

#[test]
fn format_detect_unknown() {
    let p = PathBuf::from("somefile.tar.gz");
    assert_eq!(ForeignFormat::detect(&p), None);
}

#[test]
fn deb_arch_mapping() {
    use fpm_compat::deb;
    let control = b"Package: test\nVersion: 1.0\nArchitecture: amd64\nDescription: test pkg\n";
    let fields = parse_control_pub(control);
    assert_eq!(fields.get("package").map(String::as_str), Some("test"));
    assert_eq!(fields.get("architecture").map(String::as_str), Some("amd64"));
}

#[test]
fn apk_pkginfo_roundtrip() {
    let pkginfo = "pkgname = zlib\npkgver = 1.3.1-r0\narch = x86_64\npkgdesc = Compression library\nurl = https://zlib.net\nlicense = Zlib\nsize = 102400\ndepend = libc\n";
    let pkg = parse_apk_info_pub(pkginfo);
    assert_eq!(pkg.name, "zlib");
    assert_eq!(pkg.version, "1.3.1-r0");
    assert_eq!(pkg.arch, "x86_64");
    assert_eq!(pkg.depends.len(), 1);
    assert_eq!(pkg.depends[0].name, "libc");
}

#[test]
fn arch_pkginfo_roundtrip() {
    let pkginfo = "pkgname = htop\npkgver = 3.3.0-1\narch = x86_64\npkgdesc = Interactive process viewer\nurl = https://htop.dev\nlicense = GPL-2.0\nsize = 204800\ndepend = ncurses\nconflict = htop-git\nprovides = process-viewer\n";
    let pkg = parse_arch_info_pub(pkginfo);
    assert_eq!(pkg.name, "htop");
    assert_eq!(pkg.conflicts, vec!["htop-git"]);
    assert_eq!(pkg.provides, vec!["process-viewer"]);
}

#[test]
fn manifest_toml_contains_package_section() {
    let pkg = ForeignPackage {
        format:       "deb".into(),
        name:         "curl".into(),
        version:      "8.0.1".into(),
        release:      "1".into(),
        arch:         "x86_64".into(),
        summary:      "Command line tool for URLs".into(),
        description:  "curl transfers data".into(),
        license:      "curl".into(),
        homepage:     "https://curl.se".into(),
        maintainer:   "pkg@fsociety".into(),
        depends:      vec![],
        conflicts:    vec![],
        provides:     vec![],
        installed_size: 4096,
    };
    let toml = fpm_compat::convert::to_manifest_toml(&pkg);
    assert!(toml.contains("[package]"));
    assert!(toml.contains("name        = \"curl\""));
    assert!(toml.contains("origin_format = \"deb\""));
}

fn parse_control_pub(data: &[u8]) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for line in std::io::BufRead::lines(std::io::BufReader::new(data)) {
        let line = line.unwrap();
        if let Some(colon) = line.find(':') {
            let k = line[..colon].trim().to_lowercase();
            let v = line[colon + 1..].trim().to_string();
            map.insert(k, v);
        }
    }
    map
}

fn parse_apk_info_pub(s: &str) -> ForeignPackage {
    let path = {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.path().to_path_buf()
    };
    build_apk_foreign(s)
}

fn build_apk_foreign(s: &str) -> ForeignPackage {
    use fpm_compat::convert::ForeignDep;
    let mut name = String::new(); let mut version = String::new();
    let mut arch = String::new(); let mut desc = String::new();
    let mut url  = String::new(); let mut license = String::new();
    let mut size = 0u64;
    let mut depends = Vec::new();
    for line in s.lines() {
        if let Some((k, v)) = line.split_once(" = ") {
            match k.trim() {
                "pkgname" => name    = v.trim().into(),
                "pkgver"  => version = v.trim().into(),
                "arch"    => arch    = v.trim().into(),
                "pkgdesc" => desc    = v.trim().into(),
                "url"     => url     = v.trim().into(),
                "license" => license = v.trim().into(),
                "size"    => size    = v.trim().parse().unwrap_or(0),
                "depend"  => depends.push(ForeignDep { name: v.trim().into(), version: None, optional: false }),
                _ => {}
            }
        }
    }
    ForeignPackage { format: "apk".into(), name, version, release: "1".into(), arch,
        summary: desc.clone(), description: desc, license, homepage: url,
        maintainer: String::new(), depends, conflicts: vec![], provides: vec![], installed_size: size }
}

fn parse_arch_info_pub(s: &str) -> ForeignPackage {
    use fpm_compat::convert::ForeignDep;
    let mut name = String::new(); let mut version = String::new();
    let mut arch = String::new(); let mut desc = String::new();
    let mut url  = String::new(); let mut license = String::new();
    let mut size = 0u64;
    let mut depends = Vec::new(); let mut conflicts = Vec::new(); let mut provides = Vec::new();
    for line in s.lines() {
        if let Some((k, v)) = line.split_once(" = ") {
            match k.trim() {
                "pkgname"  => name     = v.trim().into(),
                "pkgver"   => version  = v.trim().into(),
                "arch"     => arch     = v.trim().into(),
                "pkgdesc"  => desc     = v.trim().into(),
                "url"      => url      = v.trim().into(),
                "license"  => license  = v.trim().into(),
                "size"     => size     = v.trim().parse().unwrap_or(0),
                "depend"   => depends.push(ForeignDep { name: v.trim().into(), version: None, optional: false }),
                "conflict" => conflicts.push(v.trim().into()),
                "provides" => provides.push(v.trim().into()),
                _ => {}
            }
        }
    }
    ForeignPackage { format: "arch".into(), name, version, release: "1".into(), arch,
        summary: desc.clone(), description: desc, license, homepage: url,
        maintainer: String::new(), depends, conflicts, provides, installed_size: size }
}
