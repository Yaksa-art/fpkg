use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    path::PathBuf,
    sync::Arc,
    thread,
};
use tracing::{error, info, warn};
use crate::{rpc, state::DaemonState};

pub fn run_accept_loop(socket_path: PathBuf, state: Arc<DaemonState>) -> anyhow::Result<()> {
    let listener = UnixListener::bind(&socket_path)?;
    info!(socket = %socket_path.display(), "listening");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = Arc::clone(&state);
                thread::spawn(move || handle_connection(stream, state));
            }
            Err(e) => {
                warn!("accept error: {e}");
            }
        }
    }
    Ok(())
}

fn handle_connection(
    stream: std::os::unix::net::UnixStream,
    state: Arc<DaemonState>,
) {
    let mut writer = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => { error!("clone stream: {e}"); return; }
    };
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() { continue; }

        let response = rpc::dispatch::handle_line(&line, &state);
        let mut out = serde_json::to_string(&response).unwrap_or_else(|e| {
            serde_json::json!({
                "jsonrpc": "2.0",
                "error": { "code": -32700, "message": e.to_string() },
                "id": null
            }).to_string()
        });
        out.push('\n');
        if let Err(e) = writer.write_all(out.as_bytes()) {
            error!("write response: {e}");
            break;
        }
    }
}
