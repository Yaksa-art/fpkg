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

    let user_mode = params
        .get("user")
        .and_then(|v| v.as_bool())
        .unwrap_or(state.config.mode == "user");

    info!(packages = ?names, user = user_mode, "install requested");

    match do_install(&names, user_mode, state) {
        Ok(installed) => Response::ok(id, serde_json::json!({
            "installed": installed,
            "status": "ok"
        })),
        Err(e) => {
            error!(error = %e, "install failed");
            Response::err(id, -32000, e.to_string())
        }
    }
}

fn do_install(
    names: &[String],
    user_mode: bool,
    state: &DaemonState,
) -> anyhow::Result<Vec<String>> {
    let cache_dir = state.config.cache_dir.clone();
    std::fs::create_dir_all(&cache_dir)?;

    let mut installed = Vec::new();
    for name in names {
        let pkg_path = cache_dir.join(format!("{name}.fpkg"));

        if !pkg_path.exists() {
            anyhow::bail!("package not in cache: {name} (run 'update' first)");
        }

        let prefix = if user_mode {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            std::path::PathBuf::from(home).join(".local/fpm/packages").join(name)
        } else {
            std::path::PathBuf::from("/usr")
        };

        std::fs::create_dir_all(&prefix)?;

        let db = state.db.lock().unwrap();
        db.register_package(name, "unknown", &prefix.to_string_lossy())?;
        installed.push(name.clone());
        info!(pkg = %name, prefix = %prefix.display(), "installed");
    }
    Ok(installed)
}
