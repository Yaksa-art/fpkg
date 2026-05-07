use fpmd::rpc::{dispatch, Response};
use serde_json::{json, Value};

fn make_state() -> fpmd::state::DaemonState {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut cfg = fpmd::config::DaemonConfig::default();
    cfg.db_path    = tmp.path().join("db.sqlite");
    cfg.cache_dir  = tmp.path().join("cache");
    cfg.socket_path = tmp.path().join("fpmd.sock");
    fpmd::state::DaemonState::new(cfg).unwrap()
}

#[test]
fn parse_error_on_invalid_json() {
    let state = make_state();
    let resp = dispatch::handle_line("not json{", &state);
    let v = serde_json::to_value(&resp).unwrap();
    assert_eq!(v["error"]["code"], -32700);
}

#[test]
fn method_not_found() {
    let state = make_state();
    let req = json!({ "jsonrpc": "2.0", "method": "frobnicate", "id": 1 });
    let resp = dispatch::handle_line(&req.to_string(), &state);
    let v = serde_json::to_value(&resp).unwrap();
    assert_eq!(v["error"]["code"], -32601);
}

#[test]
fn status_returns_version() {
    let state = make_state();
    let req = json!({ "jsonrpc": "2.0", "method": "status", "id": 1 });
    let resp = dispatch::handle_line(&req.to_string(), &state);
    let v = serde_json::to_value(&resp).unwrap();
    assert!(v["result"]["version"].is_string());
    assert!(v["result"]["installed"].is_number());
}

#[test]
fn list_returns_array() {
    let state = make_state();
    let req = json!({ "jsonrpc": "2.0", "method": "list", "id": 2 });
    let resp = dispatch::handle_line(&req.to_string(), &state);
    let v = serde_json::to_value(&resp).unwrap();
    assert!(v["result"]["packages"].is_array());
}

#[test]
fn install_missing_packages_param() {
    let state = make_state();
    let req = json!({ "jsonrpc": "2.0", "method": "install", "params": {}, "id": 3 });
    let resp = dispatch::handle_line(&req.to_string(), &state);
    let v = serde_json::to_value(&resp).unwrap();
    assert_eq!(v["error"]["code"], -32602);
}

#[test]
fn remove_missing_packages_param() {
    let state = make_state();
    let req = json!({ "jsonrpc": "2.0", "method": "remove", "params": {}, "id": 4 });
    let resp = dispatch::handle_line(&req.to_string(), &state);
    let v = serde_json::to_value(&resp).unwrap();
    assert_eq!(v["error"]["code"], -32602);
}

#[test]
fn rollback_missing_generation_param() {
    let state = make_state();
    let req = json!({ "jsonrpc": "2.0", "method": "rollback", "params": {}, "id": 5 });
    let resp = dispatch::handle_line(&req.to_string(), &state);
    let v = serde_json::to_value(&resp).unwrap();
    assert_eq!(v["error"]["code"], -32602);
}

#[test]
fn update_returns_ok_no_repos() {
    let state = make_state();
    let req = json!({ "jsonrpc": "2.0", "method": "update", "id": 6 });
    let resp = dispatch::handle_line(&req.to_string(), &state);
    let v = serde_json::to_value(&resp).unwrap();
    assert_eq!(v["result"]["status"], "ok");
    assert!(v["result"]["synced_repos"].is_array());
}

#[test]
fn invalid_jsonrpc_version() {
    let state = make_state();
    let req = json!({ "jsonrpc": "1.0", "method": "status", "id": 7 });
    let resp = dispatch::handle_line(&req.to_string(), &state);
    let v = serde_json::to_value(&resp).unwrap();
    assert_eq!(v["error"]["code"], -32600);
}
