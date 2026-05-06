use fpm_db::{Database, DbError, QueryExt};
use fpm_db::models::DbFile;

#[test]
fn test_open_in_memory() {
    let db = Database::open_in_memory().unwrap();
    assert_eq!(db.package_count().unwrap(), 0);
}

#[test]
fn test_insert_and_get_package() {
    let db = Database::open_in_memory().unwrap();
    db.insert_generation(1, "init", "2026-01-01T00:00:00+00:00", None, "[]").unwrap();
    let id = db.insert_package("firefox", "125.0.3", 1, Some("abc123"), 82_000_000, true).unwrap();
    assert!(id > 0);
    let pkg = db.get_package("firefox").unwrap().unwrap();
    assert_eq!(pkg.name, "firefox");
    assert_eq!(pkg.version, "125.0.3");
    assert_eq!(pkg.blake3.as_deref(), Some("abc123"));
    assert!(pkg.explicit);
}

#[test]
fn test_list_packages() {
    let db = Database::open_in_memory().unwrap();
    db.insert_generation(1, "init", "2026-01-01T00:00:00+00:00", None, "[]").unwrap();
    for (name, ver) in &[("curl", "8.7.1"), ("vim", "9.1.0"), ("git", "2.44.0")] {
        db.insert_package(name, ver, 1, None, 1024, true).unwrap();
    }
    let list = db.list_packages().unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].name, "curl");
    assert_eq!(list[1].name, "git");
    assert_eq!(list[2].name, "vim");
}

#[test]
fn test_remove_package() {
    let db = Database::open_in_memory().unwrap();
    db.insert_generation(1, "init", "2026-01-01T00:00:00+00:00", None, "[]").unwrap();
    db.insert_package("vim", "9.1.0", 1, None, 4096, true).unwrap();
    let removed = db.remove_package("vim", "9.1.0").unwrap();
    assert!(removed);
    assert!(db.get_package("vim").unwrap().is_none());
}

#[test]
fn test_insert_and_query_files() {
    let db = Database::open_in_memory().unwrap();
    db.insert_generation(1, "init", "2026-01-01T00:00:00+00:00", None, "[]").unwrap();
    let pkg_id = db.insert_package("curl", "8.7.1", 1, None, 512, true).unwrap();
    let files = vec![
        DbFile { id: 0, package_id: pkg_id, package_name: "curl".into(),
                 path: "usr/bin/curl".into(), blake3: "aabb".into(), size: 256, file_type: "file".into() },
        DbFile { id: 0, package_id: pkg_id, package_name: "curl".into(),
                 path: "usr/share/man/man1/curl.1".into(), blake3: "ccdd".into(), size: 256, file_type: "file".into() },
    ];
    db.insert_files(pkg_id, "curl", &files).unwrap();
    let owned = db.owner_of("usr/bin/curl").unwrap();
    assert_eq!(owned.as_deref(), Some("curl"));
    let pkg_files = db.files_of("curl").unwrap();
    assert_eq!(pkg_files.len(), 2);
}

#[test]
fn test_owner_of_unknown_path() {
    let db = Database::open_in_memory().unwrap();
    let owner = db.owner_of("usr/bin/nonexistent").unwrap();
    assert!(owner.is_none());
}

#[test]
fn test_insert_and_list_generations() {
    let db = Database::open_in_memory().unwrap();
    db.insert_generation(1, "initial install", "2026-01-01T00:00:00+00:00", None, "[]").unwrap();
    db.insert_generation(2, "install firefox", "2026-01-02T00:00:00+00:00", Some(1), "[]").unwrap();
    db.insert_generation(3, "rollback to 1",   "2026-01-03T00:00:00+00:00", Some(2), "[]").unwrap();
    let gens = db.list_generations().unwrap();
    assert_eq!(gens.len(), 3);
    assert_eq!(gens[0].gen_id, 1);
    assert_eq!(gens[2].description, "rollback to 1");
    assert_eq!(gens[2].parent_gen_id, Some(2));
}

#[test]
fn test_hold_and_unhold() {
    let db = Database::open_in_memory().unwrap();
    db.hold("firefox", Some("manual hold")).unwrap();
    assert!(db.is_held("firefox").unwrap());
    let holds = db.list_holds().unwrap();
    assert_eq!(holds.len(), 1);
    assert_eq!(holds[0].package_name, "firefox");
    assert_eq!(holds[0].reason.as_deref(), Some("manual hold"));
    db.unhold("firefox").unwrap();
    assert!(!db.is_held("firefox").unwrap());
}

#[test]
fn test_search_by_name() {
    let db = Database::open_in_memory().unwrap();
    db.insert_generation(1, "init", "2026-01-01T00:00:00+00:00", None, "[]").unwrap();
    db.insert_package("libcurl", "8.7.1", 1, None, 0, false).unwrap();
    db.insert_package("curl",    "8.7.1", 1, None, 0, true).unwrap();
    db.insert_package("vim",     "9.1.0", 1, None, 0, true).unwrap();
    let results = db.search("curl").unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|p| p.name == "curl"));
    assert!(results.iter().any(|p| p.name == "libcurl"));
}

#[test]
fn test_upgradeable_excludes_held() {
    let db = Database::open_in_memory().unwrap();
    db.insert_generation(1, "init", "2026-01-01T00:00:00+00:00", None, "[]").unwrap();
    db.insert_package("firefox", "125.0.3", 1, None, 0, true).unwrap();
    db.insert_package("vim",     "9.1.0",   1, None, 0, true).unwrap();
    db.hold("firefox", None).unwrap();
    let up = db.upgradeable().unwrap();
    assert_eq!(up.len(), 1);
    assert_eq!(up[0].name, "vim");
}

#[test]
fn test_explicit_packages() {
    let db = Database::open_in_memory().unwrap();
    db.insert_generation(1, "init", "2026-01-01T00:00:00+00:00", None, "[]").unwrap();
    db.insert_package("firefox", "125.0.3", 1, None, 0, true).unwrap();
    db.insert_package("libffi",  "3.4.6",   1, None, 0, false).unwrap();
    let explicit = db.explicit_packages().unwrap();
    assert_eq!(explicit.len(), 1);
    assert_eq!(explicit[0].name, "firefox");
}

#[test]
fn test_stats() {
    let db = Database::open_in_memory().unwrap();
    db.insert_generation(1, "init", "2026-01-01T00:00:00+00:00", None, "[]").unwrap();
    db.insert_package("a", "1.0", 1, None, 1000, true).unwrap();
    db.insert_package("b", "1.0", 1, None, 2000, true).unwrap();
    assert_eq!(db.package_count().unwrap(), 2);
    assert_eq!(db.total_installed_size().unwrap(), 3000);
}

#[test]
fn test_repo_upsert_and_list() {
    let pool = fpm_db::open_pool_in_memory().unwrap();
    let store = fpm_db::RepoStore::new(&pool);
    store.upsert("fsociety-main", "https://repo.fsocietywiki.org/main", 1000, true, None).unwrap();
    store.upsert("fsociety-security", "https://repo.fsocietywiki.org/security", 1001, true, None).unwrap();
    let repos = store.list().unwrap();
    assert_eq!(repos.len(), 2);
    assert_eq!(repos[0].name, "fsociety-security");
    assert_eq!(repos[0].priority, 1001);
}

#[test]
fn test_repo_enable_disable() {
    let pool = fpm_db::open_pool_in_memory().unwrap();
    let store = fpm_db::RepoStore::new(&pool);
    store.upsert("test-repo", "https://example.com", 100, true, None).unwrap();
    store.set_enabled("test-repo", false).unwrap();
    assert!(store.list_enabled().unwrap().is_empty());
    store.set_enabled("test-repo", true).unwrap();
    assert_eq!(store.list_enabled().unwrap().len(), 1);
}

#[test]
fn test_repo_mark_synced() {
    let pool = fpm_db::open_pool_in_memory().unwrap();
    let store = fpm_db::RepoStore::new(&pool);
    store.upsert("test-repo", "https://example.com", 100, true, None).unwrap();
    store.mark_synced("test-repo", Some("\"abc-etag-123\"")).unwrap();
    let r = store.get("test-repo").unwrap().unwrap();
    assert_eq!(r.etag.as_deref(), Some("\"abc-etag-123\""));
    assert!(r.last_synced_at.is_some());
}

#[test]
fn test_prune_generations_keeps_n() {
    let db = Database::open_in_memory().unwrap();
    for i in 1..=8i64 {
        db.insert_generation(i, &format!("gen {}", i), "2026-01-01T00:00:00+00:00", None, "[]").unwrap();
    }
    let pruned = db.prune_generations(5).unwrap();
    assert_eq!(pruned, 3);
    let remaining = db.list_generations().unwrap();
    assert_eq!(remaining.len(), 5);
    assert_eq!(remaining[0].gen_id, 4);
    assert_eq!(remaining[4].gen_id, 8);
}

#[test]
fn test_prune_noop_when_below_limit() {
    let db = Database::open_in_memory().unwrap();
    db.insert_generation(1, "only", "2026-01-01T00:00:00+00:00", None, "[]").unwrap();
    let pruned = db.prune_generations(5).unwrap();
    assert_eq!(pruned, 0);
    assert_eq!(db.list_generations().unwrap().len(), 1);
}

#[test]
fn test_snapshot_generation() {
    let db = Database::open_in_memory().unwrap();
    db.insert_generation(1, "init", "2026-01-01T00:00:00+00:00", None, "[]").unwrap();
    db.insert_package("curl", "8.7.1", 1, None, 512, true).unwrap();
    db.insert_package("vim",  "9.1.0", 1, None, 1024, true).unwrap();
    let json = db.snapshot_generation(1).unwrap();
    assert!(json.contains("curl"));
    assert!(json.contains("vim"));
    let val = db.generation_snapshot(1).unwrap().unwrap();
    assert!(val.is_array());
    assert_eq!(val.as_array().unwrap().len(), 2);
}

#[test]
fn test_generation_chain() {
    let db = Database::open_in_memory().unwrap();
    db.insert_generation(1, "root",   "2026-01-01T00:00:00+00:00", None,    "[]").unwrap();
    db.insert_generation(2, "second", "2026-01-02T00:00:00+00:00", Some(1), "[]").unwrap();
    db.insert_generation(3, "third",  "2026-01-03T00:00:00+00:00", Some(2), "[]").unwrap();
    let chain = db.generation_chain(3).unwrap();
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0].gen_id, 3);
    assert_eq!(chain[2].gen_id, 1);
}

#[test]
fn test_latest_generation() {
    let db = Database::open_in_memory().unwrap();
    assert!(db.latest_generation().unwrap().is_none());
    db.insert_generation(1, "first",  "2026-01-01T00:00:00+00:00", None,    "[]").unwrap();
    db.insert_generation(5, "latest", "2026-01-05T00:00:00+00:00", Some(1), "[]").unwrap();
    db.insert_generation(3, "middle", "2026-01-03T00:00:00+00:00", Some(1), "[]").unwrap();
    let latest = db.latest_generation().unwrap().unwrap();
    assert_eq!(latest.gen_id, 5);
}

#[test]
fn test_concurrent_read_100_threads() {
    use std::sync::Arc;
    use std::thread;

    let pool = Arc::new(fpm_db::open_pool_in_memory().unwrap());

    {
        let conn = pool.get().unwrap();
        conn.execute_batch("
            INSERT INTO generations (gen_id, description, created_at, packages_json)
            VALUES (1, 'init', '2026-01-01T00:00:00+00:00', '[]');
            INSERT INTO packages (name, version, generation_id, installed_size, installed_at, explicit)
            VALUES ('curl', '8.7.1', 1, 512, '2026-01-01T00:00:00+00:00', 1);
        ").unwrap();
    }

    let handles: Vec<_> = (0..100)
        .map(|_| {
            let pool = Arc::clone(&pool);
            thread::spawn(move || {
                let conn = pool.get().expect("pool exhausted");
                let count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM packages", [], |r| r.get(0))
                    .expect("query failed");
                assert_eq!(count, 1);
            })
        })
        .collect();

    for h in handles {
        h.join().expect("thread panicked");
    }
}
