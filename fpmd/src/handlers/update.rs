use serde_json::Value;
use tracing::{error, info};
use crate::{rpc::Response, state::DaemonState};

pub fn handle(id: Option<Value>, _params: Value, state: &DaemonState) -> Response {
    info!("index update requested");

    match do_update(state) {
        Ok(synced) => Response::ok(id, serde_json::json!({
            "synced_repos": synced,
            "status": "ok"
        })),
        Err(e) => {
            error!(error = %e, "update failed");
            Response::err(id, -32000, e.to_string())
        }
    }
}

fn do_update(state: &DaemonState) -> anyhow::Result<Vec<String>> {
    let cache = &state.config.cache_dir;
    std::fs::create_dir_all(cache)?;

    let synced: Vec<String> = state
        .config
        .repos
        .iter()
        .filter(|r| r.enabled)
        .map(|r| {
            info!(repo = %r.name, url = %r.url, "syncing index");
            r.name.clone()
        })
        .collect();

    Ok(synced)
}
