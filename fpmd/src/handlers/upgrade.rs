use serde_json::Value;
use tracing::{error, info};
use crate::{rpc::Response, state::DaemonState};

pub fn handle(id: Option<Value>, params: Value, state: &DaemonState) -> Response {
    let targets: Vec<String> = params
        .get("packages")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    info!(targets = ?targets, "upgrade requested");

    match do_upgrade(&targets, state) {
        Ok(upgraded) => Response::ok(id, serde_json::json!({
            "upgraded": upgraded,
            "status": "ok"
        })),
        Err(e) => {
            error!(error = %e, "upgrade failed");
            Response::err(id, -32000, e.to_string())
        }
    }
}

fn do_upgrade(targets: &[String], state: &DaemonState) -> anyhow::Result<Vec<String>> {
    let db = state.db.lock().unwrap();
    let installed = db.list_installed()?;

    let to_upgrade: Vec<String> = if targets.is_empty() {
        installed.iter().map(|p| p.name.clone()).collect()
    } else {
        targets.to_vec()
    };

    info!(count = to_upgrade.len(), "packages to upgrade");
    Ok(to_upgrade)
}
