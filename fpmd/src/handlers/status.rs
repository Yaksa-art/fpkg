use serde_json::Value;
use crate::{rpc::Response, state::DaemonState};

pub fn handle(id: Option<Value>, _params: Value, state: &DaemonState) -> Response {
    let db     = state.db.lock().unwrap();
    let count  = db.list_installed().map(|v| v.len()).unwrap_or(0);

    Response::ok(id, serde_json::json!({
        "version":    env!("CARGO_PKG_VERSION"),
        "mode":       state.config.mode,
        "socket":     state.config.socket_path.display().to_string(),
        "db":         state.config.db_path.display().to_string(),
        "installed":  count,
        "repos":      state.config.repos.len(),
    }))
}
