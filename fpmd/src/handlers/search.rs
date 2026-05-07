use serde_json::Value;
use tracing::info;
use crate::{rpc::Response, state::DaemonState};

pub fn handle(id: Option<Value>, params: Value, _state: &DaemonState) -> Response {
    let query = match params.get("query").and_then(|v| v.as_str()) {
        Some(q) => q.to_string(),
        None    => return Response::invalid_params(id, "'query' string required"),
    };

    info!(query = %query, "search requested");

    Response::ok(id, serde_json::json!({
        "query":   query,
        "results": [],
        "note":    "index search not yet wired — run 'update' first"
    }))
}
