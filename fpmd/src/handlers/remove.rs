use serde_json::Value;
use tracing::{error, info};
use crate::{rpc::Response, state::DaemonState};

pub fn handle(id: Option<Value>, params: Value, state: &DaemonState) -> Response {
    let names: Vec<String> = match params.get("packages") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => return Response::invalid_params(id, "'packages' array required"),
    };

    info!(packages = ?names, "remove requested");

    match do_remove(&names, state) {
        Ok(removed) => Response::ok(id, serde_json::json!({
            "removed": removed,
            "status": "ok"
        })),
        Err(e) => {
            error!(error = %e, "remove failed");
            Response::err(id, -32000, e.to_string())
        }
    }
}

fn do_remove(names: &[String], state: &DaemonState) -> anyhow::Result<Vec<String>> {
    let db = state.db.lock().unwrap();
    let mut removed = Vec::new();
    for name in names {
        db.unregister_package(name)?;
        removed.push(name.clone());
        info!(pkg = %name, "removed");
    }
    Ok(removed)
}
