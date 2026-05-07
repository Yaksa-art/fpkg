use fpm_db::pool::open_pool_in_memory;
use fpm_db::repos::RepoStore;
use fpm_index::{
    store::IndexStore,
    syncer::{IndexSyncer, SyncOutcome},
    proto::RepoIndex,
};
use tempfile::TempDir;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

fn make_repo_index(name: &str, pkg_count: usize) -> RepoIndex {
    let packages = (0..pkg_count)
        .map(|i| fpm_index::proto::IndexPackage {
            name: format!("pkg{}", i),
            version: "1.0.0".to_string(),
            deps: vec![],
            provides: vec![],
            conflicts: vec![],
            blake3: format!("hash{}", i),
            size: 1024,
            url_path: format!("/pool/pkg{}-1.0.0.fpkg", i),
        })
        .collect();
    RepoIndex {
        repo: name.to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        packages,
    }
}

#[tokio::test]
async fn full_fetch_on_first_sync() {
    let server = MockServer::start().await;
    let index = make_repo_index("main", 3);
    let body = rmp_serde::to_vec(&index).unwrap();

    Mock::given(method("GET"))
        .and(path("/index.msgpack"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(body)
                .insert_header("etag", "\"abc123\""),
        )
        .mount(&server)
        .await;

    let tmp = TempDir::new().unwrap();
    let store = IndexStore::new(tmp.path());
    let syncer = IndexSyncer::new(store);
    let pool = open_pool_in_memory().unwrap();

    let repos = RepoStore::new(&pool);
    repos.upsert("main", &server.uri(), 100, true, None).unwrap();

    let outcome = syncer
        .sync_repo("main", &server.uri(), None, &pool)
        .await
        .unwrap();

    assert_eq!(outcome, SyncOutcome::Created);

    let loaded = syncer.load_index("main").unwrap().unwrap();
    assert_eq!(loaded.versions_of("pkg0").len(), 1);
}

#[tokio::test]
async fn not_modified_on_matching_etag() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/index.msgpack"))
        .respond_with(ResponseTemplate::new(304))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/index.delta.msgpack"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let tmp = TempDir::new().unwrap();
    let index = make_repo_index("main", 2);
    let store = IndexStore::new(tmp.path());
    store.save("main", &index).unwrap();
    let syncer = IndexSyncer::new(store);
    let pool = open_pool_in_memory().unwrap();

    let repos = RepoStore::new(&pool);
    repos.upsert("main", &server.uri(), 100, true, None).unwrap();

    let outcome = syncer
        .sync_repo("main", &server.uri(), Some("\"v1\""), &pool)
        .await
        .unwrap();

    assert_eq!(outcome, SyncOutcome::NotModified);
}
