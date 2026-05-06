use fpm_core::{
    generation::{Generation, GenerationMeta, GenerationPackage},
    paths::FpmPaths,
    plan::{InstallPlan, PlanOp},
    trx::TransactionManager,
    FetchResult,
    ResolvedPackage,
};
use std::collections::HashMap;
use tempfile::TempDir;

/// Build a FpmPaths rooted in a temp directory.
fn tmp_paths(tmp: &TempDir) -> FpmPaths {
    FpmPaths {
        lib_dir:   tmp.path().join("lib"),
        cache_dir: tmp.path().join("cache"),
        log_dir:   tmp.path().join("log"),
    }
}

#[test]
fn test_begin_commit_creates_generation() {
    let tmp = TempDir::new().unwrap();
    let mgr = TransactionManager::new(tmp_paths(&tmp));

    let trx = mgr.begin("install hello 1.0.0").unwrap();
    let id = trx.id;
    assert_eq!(id, 1);

    // commit — pending -> generations/1
    trx.commit().unwrap();

    let paths = tmp_paths(&tmp);
    let gen = Generation::load(&paths, 1).unwrap();
    assert_eq!(gen.meta.id, 1);
    assert_eq!(gen.meta.description, "install hello 1.0.0");
}

#[test]
fn test_current_symlink_updated_after_commit() {
    let tmp = TempDir::new().unwrap();
    let mgr = TransactionManager::new(tmp_paths(&tmp));

    mgr.begin("install a").unwrap().commit().unwrap();
    mgr.begin("install b").unwrap().commit().unwrap();

    let current = Generation::current_id(&tmp_paths(&tmp)).unwrap();
    assert_eq!(current, 2);
}

#[test]
fn test_abort_removes_pending() {
    let tmp = TempDir::new().unwrap();
    let mgr = TransactionManager::new(tmp_paths(&tmp));

    let trx = mgr.begin("install bad").unwrap();
    let pending = trx.pending_dir.clone();
    assert!(pending.exists());

    trx.abort().unwrap();
    assert!(!pending.exists());
}

#[test]
fn test_double_begin_fails() {
    let tmp = TempDir::new().unwrap();
    let mgr = TransactionManager::new(tmp_paths(&tmp));

    // First transaction — NOT committed, so pending/ still exists
    let trx = mgr.begin("first").unwrap();

    // Second begin should fail
    let result = mgr.begin("second");
    assert!(result.is_err(), "second begin should fail while pending exists");

    trx.abort().unwrap();
}

#[test]
fn test_rollback_creates_new_generation() {
    let tmp = TempDir::new().unwrap();
    let mgr = TransactionManager::new(tmp_paths(&tmp));

    mgr.begin("install firefox").unwrap().commit().unwrap(); // gen 1
    mgr.begin("install vim").unwrap().commit().unwrap();     // gen 2

    let paths = tmp_paths(&tmp);
    assert_eq!(Generation::current_id(&paths).unwrap(), 2);

    // Roll back to gen 1
    let rollback_id = mgr.rollback(1).unwrap();
    assert_eq!(rollback_id, 3); // new gen created

    let current = Generation::current_id(&paths).unwrap();
    assert_eq!(current, 3);

    let gen3 = Generation::load(&paths, 3).unwrap();
    assert!(gen3.meta.description.contains("rollback to generation 1"));
    assert_eq!(gen3.meta.parent, Some(2));
}

#[test]
fn test_rollback_to_current_fails() {
    let tmp = TempDir::new().unwrap();
    let mgr = TransactionManager::new(tmp_paths(&tmp));

    mgr.begin("init").unwrap().commit().unwrap();
    let r = mgr.rollback(1);
    assert!(r.is_err());
}

#[test]
fn test_prune_keeps_n_generations() {
    let tmp = TempDir::new().unwrap();
    let mgr = TransactionManager::new(tmp_paths(&tmp));

    for i in 1..=6 {
        mgr.begin(format!("install pkg-{}", i)).unwrap().commit().unwrap();
    }

    let paths = tmp_paths(&tmp);
    assert_eq!(Generation::list_ids(&paths).unwrap().len(), 6);

    // Keep 3; current is 6, so prune 1,2,3
    let pruned = mgr.prune(3).unwrap();
    assert_eq!(pruned.len(), 3);
    assert!(pruned.contains(&1));
    assert!(pruned.contains(&2));
    assert!(pruned.contains(&3));

    // gen 6 (current) must still exist
    assert!(Generation::load(&paths, 6).is_ok());
}

#[test]
fn test_install_plan_from_resolved() {
    let resolved = vec![
        ResolvedPackage { name: "hello".into(), version: "1.0.0".parse().unwrap(), blake3: None, deps: vec![] },
        ResolvedPackage { name: "glibc".into(), version: "2.35.0".parse().unwrap(), blake3: None, deps: vec![] },
    ];
    let fetch_results = vec![
        FetchResult { package: "hello".into(), version: "1.0.0".into(), path: "/cache/hello-1.0.0.fpkg".into(), was_cached: false },
    ];
    let mut installed = HashMap::new();
    installed.insert("glibc".to_string(), "2.35.0".to_string());

    let plan = InstallPlan::from_resolved(&resolved, &fetch_results, &installed, "install hello");

    let hello = plan.entries.iter().find(|e| e.name == "hello").unwrap();
    assert_eq!(hello.op, PlanOp::Install);
    assert!(hello.fpkg_path.is_some());

    let glibc = plan.entries.iter().find(|e| e.name == "glibc").unwrap();
    assert_eq!(glibc.op, PlanOp::AlreadyInstalled);

    assert_eq!(plan.actionable().count(), 1);
}

#[test]
fn test_generation_packages_saved_in_commit() {
    let tmp = TempDir::new().unwrap();
    let mgr = TransactionManager::new(tmp_paths(&tmp));

    let resolved = vec![
        ResolvedPackage { name: "firefox".into(), version: "125.0.3".parse().unwrap(), blake3: Some("abc123".into()), deps: vec![] },
    ];
    let fetch_results = vec![
        FetchResult { package: "firefox".into(), version: "125.0.3".into(), path: "/cache/firefox.fpkg".into(), was_cached: false },
    ];

    let plan = InstallPlan::from_resolved(&resolved, &fetch_results, &HashMap::new(), "install firefox");

    let mut trx = mgr.begin("install firefox").unwrap();
    trx.set_plan(plan);
    let id = trx.commit().unwrap();

    let gen = Generation::load(&tmp_paths(&tmp), id).unwrap();
    assert_eq!(gen.meta.packages.len(), 1);
    assert_eq!(gen.meta.packages[0].name, "firefox");
    assert_eq!(gen.meta.packages[0].blake3.as_deref(), Some("abc123"));
}
