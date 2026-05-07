use serde_json::Value;
use tracing::{error, info};
use crate::{rpc::Response, state::DaemonState};

pub fn handle(id: Option<Value>, params: Value, state: &DaemonState) -> Response {
    let gen_id = match params.get("generation").and_then(|v| v.as_i64()) {
        Some(g) => g as i32,
        None    => return Response::invalid_params(id, "'generation' (integer) required"),
    };

    info!(generation = gen_id, "rollback requested");

    match do_rollback(gen_id, state) {
        Ok(()) => Response::ok(id, serde_json::json!({
            "rolled_back_to": gen_id,
            "status": "ok"
        })),
        Err(e) => {
            error!(error = %e, "rollback failed");
            Response::err(id, -32000, e.to_string())
        }
    }
}

fn do_rollback(gen_id: i32, state: &DaemonState) -> anyhow::Result<()> {
    let db = state.db.lock().unwrap();
    db.rollback_to_generation(gen_id)?;
    info!(generation = gen_id, "rollback complete");
    Ok(())
}
