//! Integration tests for M5 Installer.
//!
//! Tests create real .fpkg archives (tar.zst) in a tempdir and
//! run the full extract → manifest → remove pipeline.

use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
};

use fpm_core::{
    plan::{InstallPlan, PlanEntry, PlanOp},
    trx::TransactionManager,
    paths::FpmPaths,
};
use fpm_installer::{
    extract::extract_data,
    installer::Installer,
    manifest::PackageManifest,
    remove::Remover,
};
use tempfile::TempDir;

// ---- helpers ----------------------------------------------------------------

fn tmp_paths(tmp: &TempDir) -> FpmPaths {
    FpmPaths {
        lib_dir:   tmp.path().join("lib"),
        cache_dir: tmp.path().join("cache"),
        log_dir:   tmp.path().join("log"),
    }
}

/// Build a minimal .fpkg (tar.zst) with the given DATA/ files.
/// `files` is a list of (relative path inside DATA/, content bytes).
fn make_fpkg(tmp: &TempDir, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let fpkg_path = tmp.path().join(format!("{}.fpkg", name));
    let file = std::fs::File::create(&fpkg_path).unwrap();
    let encoder = zstd::Encoder::new(file, 1).unwrap().auto_finish();
    let mut ar = tar::Builder::new(encoder);

    for (rel, content) in files {
        let entry_path = format!("DATA/{}", rel);
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        ar.append_data(&mut header, &entry_path, *content).unwrap();
    }
    ar.finish().unwrap();
    fpkg_path
}

// ---- tests ------------------------------------------------------------------

#[test]
fn test_extract_simple_files() {
    let tmp = TempDir::new().unwrap();
    let fpkg = make_fpkg(&tmp, "hello", &[
        ("usr/bin/hello", b"#!/bin/sh\necho hello"),
        ("usr/share/doc/hello/README", b"Hello package"),
    ]);

    let dest = tmp.path().join("root");
    std::fs::create_dir_all(&dest).unwrap();
    let files = extract_data(&fpkg, &dest).unwrap();

    assert_eq!(files.len(), 2);
    assert!(dest.join("usr/bin/hello").exists());
    assert!(dest.join("usr/share/doc/hello/README").exists());
}

#[test]
fn test_extract_blake3_hash_correct() {
    let tmp = TempDir::new().unwrap();
    let content = b"hello world";
    let fpkg = make_fpkg(&tmp, "hashtest", &[("usr/bin/hashtest", content)]);

    let dest = tmp.path().join("root");
    std::fs::create_dir_all(&dest).unwrap();
    let files = extract_data(&fpkg, &dest).unwrap();

    assert_eq!(files.len(), 1);
    // Verify BLAKE3
    let expected = hex::encode(blake3::hash(content).as_bytes());
    assert_eq!(files[0].blake3, expected);
}

#[test]
fn test_extract_skips_meta() {
    let tmp = TempDir::new().unwrap();
    // fpkg with both META/ and DATA/ entries
    let fpkg_path = tmp.path().join("mixed.fpkg");
    let file = std::fs::File::create(&fpkg_path).unwrap();
    let encoder = zstd::Encoder::new(file, 1).unwrap().auto_finish();
    let mut ar = tar::Builder::new(encoder);

    for (path, content) in &[
        ("META/manifest.toml", b"[package]\nname=\"mixed\"" as &[u8]),
        ("DATA/usr/bin/myapp", b"binary"),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        ar.append_data(&mut header, path, *content).unwrap();
    }
    ar.finish().unwrap();

    let dest = tmp.path().join("root");
    std::fs::create_dir_all(&dest).unwrap();
    let files = extract_data(&fpkg_path, &dest).unwrap();

    // Only DATA/usr/bin/myapp should be extracted
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].rel_path.to_string_lossy(), "usr/bin/myapp");
    // META/manifest.toml must NOT appear in dest
    assert!(!dest.join("META").exists());
}

#[test]
fn test_manifest_save_load_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");

    let fpkg = make_fpkg(&tmp, "pkg", &[
        ("usr/bin/pkg", b"binary"),
        ("usr/share/pkg/data.txt", b"data"),
    ]);
    let files = extract_data(&fpkg, &root).unwrap();

    let mut manifest = PackageManifest::new("pkg", "1.0.0");
    manifest.add_from_extracted(&files);
    manifest.save(&root).unwrap();

    let loaded = PackageManifest::load(&root, "pkg", "1.0.0").unwrap();
    assert_eq!(loaded.name, "pkg");
    assert_eq!(loaded.version, "1.0.0");
    assert_eq!(loaded.files.len(), 2);
}

#[test]
fn test_installer_without_hooks() {
    let tmp = TempDir::new().unwrap();
    let paths = tmp_paths(&tmp);
    let mgr = TransactionManager::new(paths.clone());

    let fpkg = make_fpkg(&tmp, "hello", &[
        ("usr/bin/hello", b"#!/bin/sh\necho hi"),
    ]);

    let entries = vec![
        PlanEntry {
            name: "hello".into(),
            version: "1.0.0".into(),
            op: PlanOp::Install,
            fpkg_path: Some(fpkg.clone()),
            blake3: None,
            installed_size: None,
        },
    ];
    let plan = InstallPlan { entries, description: "install hello".into() };

    let mut trx = mgr.begin("install hello").unwrap();
    trx.set_plan(plan.clone());

    let installer = Installer::without_hooks();
    let result = installer.install_plan(&trx, &plan).unwrap();

    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].files_installed, 1);
    assert!(trx.root_dir().join("usr/bin/hello").exists());

    // Commit
    let gen_id = trx.commit().unwrap();
    assert_eq!(gen_id, 1);
}

#[test]
fn test_installer_skips_already_installed() {
    let tmp = TempDir::new().unwrap();
    let paths = tmp_paths(&tmp);
    let mgr = TransactionManager::new(paths.clone());
    let fpkg = make_fpkg(&tmp, "pkg", &[("usr/bin/pkg", b"bin")]);

    let entries = vec![
        PlanEntry {
            name: "pkg".into(),
            version: "1.0.0".into(),
            op: PlanOp::AlreadyInstalled,
            fpkg_path: Some(fpkg.clone()),
            blake3: None,
            installed_size: None,
        },
    ];
    let plan = InstallPlan { entries, description: "noop".into() };
    let trx = mgr.begin("noop").unwrap();

    let installer = Installer::without_hooks();
    let result = installer.install_plan(&trx, &plan).unwrap();

    assert_eq!(result.records.len(), 0);
    assert_eq!(result.skipped.len(), 1);
    trx.abort().unwrap();
}

#[test]
fn test_remover_deletes_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");

    let fpkg = make_fpkg(&tmp, "bye", &[
        ("usr/bin/bye", b"bye"),
        ("usr/share/bye/readme", b"readme"),
    ]);
    let files = extract_data(&fpkg, &root).unwrap();

    let mut manifest = PackageManifest::new("bye", "1.0.0");
    manifest.add_from_extracted(&files);
    manifest.save(&root).unwrap();

    let remover = Remover::new(&root);
    let deleted = remover.remove("bye", "1.0.0").unwrap();
    assert_eq!(deleted, 2);
    assert!(!root.join("usr/bin/bye").exists());
}

#[test]
fn test_file_conflict_detected() {
    let tmp = TempDir::new().unwrap();
    let paths = tmp_paths(&tmp);
    let mgr = TransactionManager::new(paths.clone());

    // Two packages claiming the same file
    let fpkg_a = make_fpkg(&tmp, "pkga", &[("usr/bin/conflict", b"a")]);
    let fpkg_b = make_fpkg(&tmp, "pkgb", &[("usr/bin/conflict", b"b")]);

    let entries = vec![
        PlanEntry {
            name: "pkga".into(), version: "1.0.0".into(), op: PlanOp::Install,
            fpkg_path: Some(fpkg_a), blake3: None, installed_size: None,
        },
        PlanEntry {
            name: "pkgb".into(), version: "1.0.0".into(), op: PlanOp::Install,
            fpkg_path: Some(fpkg_b), blake3: None, installed_size: None,
        },
    ];
    let plan = InstallPlan { entries, description: "conflict test".into() };
    let trx = mgr.begin("conflict test").unwrap();

    let installer = Installer { run_hooks: false, check_conflicts: true };
    let result = installer.install_plan(&trx, &plan);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(err, fpm_installer::InstallerError::FileConflict { .. }));

    trx.abort().unwrap();
}

#[test]
fn test_list_installed() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");

    for (name, ver) in &[("vim", "9.1.0"), ("curl", "8.7.1")] {
        let fpkg = make_fpkg(&tmp, name, &[("usr/bin/prog", b"bin")]);
        let files = extract_data(&fpkg, &root).unwrap();
        let mut m = PackageManifest::new(*name, *ver);
        m.add_from_extracted(&files);
        m.save(&root).unwrap();
    }

    let remover = Remover::new(&root);
    let mut installed = remover.list_installed();
    installed.sort();
    assert_eq!(installed.len(), 2);
    assert!(installed.iter().any(|(n, _)| n == "curl"));
    assert!(installed.iter().any(|(n, _)| n == "vim"));
}
