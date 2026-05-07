use serde_json::Value;
use tracing::{error, info};
use crate::{rpc::Response, state::DaemonState};

pub fn handle(id: Option<Value>, _params: Value, state: &DaemonState) -> Response {
    info!("list installed requested");

    match do_list(state) {
        Ok(pkgs) => Response::ok(id, serde_json::json!({ "packages": pkgs })),
        Err(e) => {
            error!(error = %e, "list failed");
            Response::err(id, -32000, e.to_string())
        }
    }
}

fn do_list(state: &DaemonState) -> anyhow::Result<Vec<serde_json::Value>> {
    let db = state.db.lock().unwrap();
    let installed = db.list_installed()?;
    let result = installed
        .into_iter()
        .map(|p| serde_json::json!({
            "name":    p.name,
            "version": p.version,
            "arch":    p.arch,
        }))
        .collect();
    Ok(result)
}
