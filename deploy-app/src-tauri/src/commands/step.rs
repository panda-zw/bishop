//! Generic streamer for `./deploy <subcommand> <env>` invocations — used by
//! the setup wizard for `setup-server` and `setup-app`. Same shape as the
//! health-check runner but with an arbitrary (validated) subcommand argument.
//!
//! Events: `step:<id>` for each line, `step:<id>:done` with `{ ok, code }`.

use crate::config::project::load_project;
use crate::deploy_script;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Default)]
pub struct StepStreams {
    inner: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
}

#[derive(serde::Serialize, Clone)]
struct StepDone { ok: bool, code: Option<i32> }

/// Whitelist of CLI subcommands the UI is allowed to invoke through this
/// generic runner. Extend deliberately — this is a shell boundary.
const ALLOWED: &[&str] = &["setup-server", "setup-app", "setup-github"];

#[tauri::command]
pub async fn start_cli_step(
    app: AppHandle,
    streams: State<'_, StepStreams>,
    project_path: String,
    subcommand: String,
    env: String,
) -> Result<String, String> {
    if !ALLOWED.contains(&subcommand.as_str()) {
        return Err(format!("subcommand '{}' is not allowed", subcommand));
    }

    let project_dir = PathBuf::from(&project_path);
    let script = deploy_script::resolve(&app, &project_dir)
        .map_err(|e| format!("{:#}", e))?;

    let proj = load_project(&project_dir).map_err(|e| format!("{:#}", e))?;
    if !proj.remotes.iter().any(|e| e.name == env) {
        return Err(format!("environment '{}' not found", env));
    }

    let stream_id = Uuid::new_v4().to_string();
    let event = format!("step:{}", stream_id);
    let done_event = format!("step:{}:done", stream_id);

    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    streams.inner.lock().insert(stream_id.clone(), cancel_tx);

    let app_handle = app.clone();
    tokio::spawn(async move {
        let mut child = match Command::new(&script)
            .arg(&subcommand)
            .arg(&env)
            .current_dir(&project_dir)
            .env("DEPLOY_PROJECT_ROOT", &project_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app_handle.emit(&event, format!("[spawn error: {}]", e));
                let _ = app_handle.emit(&done_event, StepDone { ok: false, code: None });
                return;
            }
        };

        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");

        let app1 = app_handle.clone(); let e1 = event.clone();
        let t1 = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => { trim_eol(&mut line); let _ = app1.emit(&e1, line); }
                }
            }
        });
        let app2 = app_handle.clone(); let e2 = event.clone();
        let t2 = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => { trim_eol(&mut line); let _ = app2.emit(&e2, line); }
                }
            }
        });

        let status = tokio::select! {
            _ = cancel_rx => { let _ = child.start_kill(); child.wait().await.ok() }
            s = child.wait() => s.ok(),
        };
        let _ = t1.await; let _ = t2.await;

        let ok = status.as_ref().map(|s| s.success()).unwrap_or(false);
        let code = status.and_then(|s| s.code());
        let _ = app_handle.emit(&done_event, StepDone { ok, code });
    });

    Ok(stream_id)
}

#[tauri::command]
pub async fn cancel_cli_step(streams: State<'_, StepStreams>, stream_id: String) -> Result<(), String> {
    if let Some(tx) = streams.inner.lock().remove(&stream_id) { let _ = tx.send(()); }
    Ok(())
}

fn trim_eol(s: &mut String) {
    if s.ends_with('\n') { s.pop(); }
    if s.ends_with('\r') { s.pop(); }
}
