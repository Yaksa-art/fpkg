use serde_json::Value;
use tracing::{debug, warn};
use crate::{rpc::{Request, Response}, state::DaemonState};
use crate::handlers;

pub fn handle_line(line: &str, state: &DaemonState) -> Response {
    let req: Request = match serde_json::from_str(line) {
        Ok(r)  => r,
        Err(e) => {
            warn!("JSON parse error: {e}");
            return Response::parse_error();
        }
    };

    if req.jsonrpc != "2.0" {
        return Response::err(req.id, -32600, "Invalid Request: jsonrpc must be \"2.0\"");
    }

    debug!(method = %req.method, "dispatch");

    let params = req.params.unwrap_or(Value::Null);
    let id     = req.id.clone();

    match req.method.as_str() {
        "install"  => handlers::install::handle(id, params, state),
        "remove"   => handlers::remove::handle(id, params, state),
        "upgrade"  => handlers::upgrade::handle(id, params, state),
        "rollback" => handlers::rollback::handle(id, params, state),
        "search"   => handlers::search::handle(id, params, state),
        "list"     => handlers::list::handle(id, params, state),
        "update"   => handlers::update::handle(id, params, state),
        "status"   => handlers::status::handle(id, params, state),
        other      => Response::method_not_found(req.id, other),
    }
}
