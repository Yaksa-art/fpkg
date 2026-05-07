use std::{fs, path::PathBuf};
use fpkg_build::pkgbuild::PkgBuild;

const MINIMAL_PKGBUILD: &str = r#"
[package]
name    = "hello"
version = "1.0.0"
release = 1
arch    = ["x86_64", "aarch64"]
license = "MIT"
summary = "Hello world package"

[build]
script = """
#!/bin/sh
mkdir -p $DESTDIR/usr/bin
echo '#!/bin/sh\necho hello' > $DESTDIR/usr/bin/hello
chmod +x $DESTDIR/usr/bin/hello
"""
"#;

const FULL_PKGBUILD: &str = r#"
[package]
name        = "htop"
version     = "3.3.0"
release     = 1
arch        = ["x86_64"]
license     = "GPL-2.0"
summary     = "Interactive process viewer"
homepage    = "https://htop.dev"
maintainer  = "pkg@fsociety"

[build]
build_depends = ["gcc", "make", "autoconf", "ncurses-dev"]
script = """
#!/bin/sh
cd /build && make -j$(nproc)
"""
package_install = """
make DESTDIR=$DESTDIR install
"""

[[runtime.requires]]
name    = "ncurses"
version = ">= 6.0"

[[runtime.requires]]
name     = "libc"
optional = false
"#;

#[test]
fn pkgbuild_minimal_parses() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), MINIMAL_PKGBUILD).unwrap();
    let pb = PkgBuild::load(tmp.path()).unwrap();
    assert_eq!(pb.package.name, "hello");
    assert_eq!(pb.package.version, "1.0.0");
    assert_eq!(pb.package.release, 1);
    assert_eq!(pb.package.arch, vec!["x86_64", "aarch64"]);
}

#[test]
fn pkgbuild_full_parses() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), FULL_PKGBUILD).unwrap();
    let pb = PkgBuild::load(tmp.path()).unwrap();
    assert_eq!(pb.package.name, "htop");
    let rt = pb.runtime.as_ref().unwrap();
    let requires = rt.requires.as_ref().unwrap();
    assert_eq!(requires.len(), 2);
    assert_eq!(requires[0].name, "ncurses");
    assert_eq!(requires[1].name, "libc");
}

#[test]
fn pkgbuild_missing_name_errors() {
    let bad = r#"
[package]
version = "1.0"
release = 1
arch    = ["x86_64"]
license = "MIT"

[build]
script = "echo ok"
"#;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), bad).unwrap();
    let err = PkgBuild::load(tmp.path()).unwrap_err();
    assert!(err.to_string().contains("package.name"));
}

#[test]
fn pkgbuild_empty_script_errors() {
    let bad = r#"
[package]
name    = "test"
version = "1.0"
release = 1
arch    = ["x86_64"]
license = "MIT"

[build]
script = "   "
"#;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), bad).unwrap();
    let err = PkgBuild::load(tmp.path()).unwrap_err();
    assert!(err.to_string().contains("build.script"));
}

#[test]
fn fpkg_name_format() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), MINIMAL_PKGBUILD).unwrap();
    let pb = PkgBuild::load(tmp.path()).unwrap();
    assert_eq!(pb.fpkg_name(), "hello-1.0.0-1.x86_64.fpkg");
}

#[test]
fn destdir_collected_and_packed() {
    let tmp_work = tempfile::TempDir::new().unwrap();
    let destdir  = tmp_work.path().join("pkg");
    let src_dir  = tmp_work.path().join("src");
    let build_dir = tmp_work.path().join("build");
    let script_path = tmp_work.path().join("build.sh");
    fs::create_dir_all(&destdir.join("usr/bin")).unwrap();
    fs::write(destdir.join("usr/bin/hello"), b"#!/bin/sh\necho hello").unwrap();
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&build_dir).unwrap();
    fs::write(&script_path, b"echo ok").unwrap();

    let env = fpkg_build::prepare::BuildEnv {
        build_dir, src_dir, destdir, script_path,
    };

    let out_dir = tempfile::TempDir::new().unwrap();
    let tmp_pb  = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp_pb.path(), MINIMAL_PKGBUILD).unwrap();
    let pb = PkgBuild::load(tmp_pb.path()).unwrap();

    let result = fpkg_build::packer::pack(&pb, &env, out_dir.path()).unwrap();
    assert_eq!(result.file_count, 1);
    assert!(result.fpkg_path.exists());
}

#[test]
fn fpkg_zip_contains_meta_and_data() {
    let tmp_work  = tempfile::TempDir::new().unwrap();
    let destdir   = tmp_work.path().join("pkg");
    let src_dir   = tmp_work.path().join("src");
    let build_dir = tmp_work.path().join("build");
    let script_path = tmp_work.path().join("build.sh");
    fs::create_dir_all(&destdir.join("usr/lib")).unwrap();
    fs::write(destdir.join("usr/lib/libtest.so"), b"ELF").unwrap();
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&build_dir).unwrap();
    fs::write(&script_path, b"echo ok").unwrap();

    let env = fpkg_build::prepare::BuildEnv {
        build_dir, src_dir, destdir, script_path,
    };
    let out_dir = tempfile::TempDir::new().unwrap();
    let tmp_pb  = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp_pb.path(), MINIMAL_PKGBUILD).unwrap();
    let pb = PkgBuild::load(tmp_pb.path()).unwrap();

    let result = fpkg_build::packer::pack(&pb, &env, out_dir.path()).unwrap();

    let f   = fs::File::open(&result.fpkg_path).unwrap();
    let mut zip = zip::ZipArchive::new(f).unwrap();
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();
    assert!(names.iter().any(|n| n == "META/manifest.toml"));
    assert!(names.iter().any(|n| n == "META/checksums.blake3"));
    assert!(names.iter().any(|n| n.starts_with("DATA/")));
}

#[test]
fn checksums_file_contains_blake3_hashes() {
    let tmp_work  = tempfile::TempDir::new().unwrap();
    let destdir   = tmp_work.path().join("pkg");
    let src_dir   = tmp_work.path().join("src");
    let build_dir = tmp_work.path().join("build");
    let script_path = tmp_work.path().join("build.sh");
    fs::create_dir_all(&destdir.join("etc")).unwrap();
    fs::write(destdir.join("etc/config.toml"), b"[cfg]
").unwrap();
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&build_dir).unwrap();
    fs::write(&script_path, b"echo ok").unwrap();

    let env = fpkg_build::prepare::BuildEnv {
        build_dir, src_dir, destdir, script_path,
    };
    let out_dir = tempfile::TempDir::new().unwrap();
    let tmp_pb  = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp_pb.path(), MINIMAL_PKGBUILD).unwrap();
    let pb = PkgBuild::load(tmp_pb.path()).unwrap();

    let result = fpkg_build::packer::pack(&pb, &env, out_dir.path()).unwrap();
    assert!(result.blake3_manifest.contains("DATA/"));
    let hash_len = result.blake3_manifest.split_whitespace().next().unwrap().len();
    assert_eq!(hash_len, 64);
}
