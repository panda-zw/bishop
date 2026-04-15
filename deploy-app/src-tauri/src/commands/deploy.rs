//! Deploy + restart commands.
//!
//! Deploy shells out to the project's `./deploy <env>` script (we don't reimplement
//! it — the script is the canonical engine). Stdout+stderr are merged and streamed
//! as `deploy:<stream_id>` Tauri events. A final `deploy:<stream_id>:done` event
//! carries the exit status.
//!
//! Restart hits the remote directly via SSH, matching the CLI's restart command.

use crate::config::project::load_project;
use crate::db::Db;
use crate::deploy_script;
use crate::ssh;
use chrono::Utc;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Default)]
pub struct DeployStreams {
    inner: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
}

#[tauri::command]
pub async fn start_deploy(
    app: AppHandle,
    streams: State<'_, DeployStreams>,
    project_path: String,
    env: String,
) -> Result<String, String> {
    let project_dir = PathBuf::from(&project_path);
    let script = deploy_script::resolve(&app, &project_dir)
        .map_err(|e| format!("{:#}", e))?;

    // Validate env name is real.
    let proj = load_project(&project_dir).map_err(|e| format!("{:#}", e))?;
    if !proj.remotes.iter().any(|e| e.name == env) {
        return Err(format!("environment '{}' not found", env));
    }

    let stream_id = Uuid::new_v4().to_string();
    let event = format!("deploy:{}", stream_id);
    let done_event = format!("deploy:{}:done", stream_id);

    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    streams.inner.lock().insert(stream_id.clone(), cancel_tx);

    let started_at = Utc::now();
    let db = app.state::<Db>().inner().clone();
    let deploy_id = match db.insert_start(&project_path, &env, started_at) {
        Ok(id) => Some(id),
        Err(e) => { tracing::warn!("failed to record deploy start: {}", e); None }
    };

    let log_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

    let app_handle = app.clone();
    let log_buf_c = log_buf.clone();
    tokio::spawn(async move {
        let mut child = match Command::new(&script)
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
                let _ = app_handle.emit(&done_event, DeployDone { ok: false, code: None });
                return;
            }
        };

        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");

        let app_stdout = app_handle.clone();
        let event_out = event.clone();
        let buf_out = log_buf_c.clone();
        let stdout_task = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        trim_eol(&mut line);
                        { let mut b = buf_out.lock(); b.push_str(&line); b.push('\n'); }
                        let _ = app_stdout.emit(&event_out, line);
                    }
                }
            }
        });

        let app_stderr = app_handle.clone();
        let event_err = event.clone();
        let buf_err = log_buf_c.clone();
        let stderr_task = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        trim_eol(&mut line);
                        { let mut b = buf_err.lock(); b.push_str(&line); b.push('\n'); }
                        let _ = app_stderr.emit(&event_err, line);
                    }
                }
            }
        });

        let mut cancelled = false;
        let status = tokio::select! {
            _ = cancel_rx => {
                cancelled = true;
                let _ = child.start_kill();
                child.wait().await.ok()
            }
            s = child.wait() => s.ok(),
        };

        let _ = stdout_task.await;
        let _ = stderr_task.await;

        let ok = !cancelled && status.as_ref().map(|s| s.success()).unwrap_or(false);
        let code = status.and_then(|s| s.code());
        let status_str = if cancelled { "cancelled" } else if ok { "success" } else { "failed" };

        if let Some(id) = deploy_id {
            let log = log_buf_c.lock().clone();
            if let Err(e) = db.update_finish(id, Utc::now(), status_str, code, &log) {
                tracing::warn!("failed to record deploy finish: {}", e);
            }
        }

        notify_deploy_done(&app_handle, &env, status_str);

        let _ = app_handle.emit(&done_event, DeployDone { ok, code });
    });

    Ok(stream_id)
}

#[tauri::command]
pub async fn cancel_deploy(streams: State<'_, DeployStreams>, stream_id: String) -> Result<(), String> {
    if let Some(tx) = streams.inner.lock().remove(&stream_id) {
        let _ = tx.send(());
    }
    Ok(())
}

#[tauri::command]
pub async fn restart_service(project_path: String, env: String, service: String) -> Result<(), String> {
    let proj = load_project(&PathBuf::from(&project_path)).map_err(|e| format!("{:#}", e))?;
    let environment = proj.remotes.into_iter().find(|e| e.name == env)
        .ok_or_else(|| format!("environment '{}' not found", env))?;

    // Accept either a bare compose service name ("app", "postgres") or a full
    // container name ("myapp-staging-worker") and strip the app_name prefix so
    // `docker compose restart` receives the compose service name it expects.
    let prefix = format!("{}-", environment.app_name);
    let service = service.strip_prefix(&prefix).unwrap_or(&service).to_string();

    let svc = shell_quote(&service);
    let app_dir = &environment.app_dir;
    let cmd = format!(
        "cd {dir} && set -a && source .env && set +a && docker compose -f docker-compose.prod.yml restart {svc}",
        dir = shell_quote(app_dir),
        svc = svc,
    );
    ssh::exec(&environment.ssh_user, &environment.ssh_host, &cmd).await
        .map(|_| ())
        .map_err(|e| format!("{:#}", e))
}

#[derive(serde::Serialize, Clone)]
struct DeployDone {
    ok: bool,
    code: Option<i32>,
}

/// Fire an OS notification only when the main window isn't currently in the
/// foreground — avoids redundant alerts when the user is watching the modal.
fn notify_deploy_done(app: &AppHandle, env: &str, status: &str) {
    let visible_and_focused = app
        .get_webview_window("main")
        .map(|w| matches!((w.is_visible(), w.is_focused()), (Ok(true), Ok(true))))
        .unwrap_or(false);
    if visible_and_focused { return; }

    let (title, body) = match status {
        "success"   => ("Deploy succeeded", format!("{} is live.", env)),
        "failed"    => ("Deploy failed",    format!("{} returned a non-zero exit.", env)),
        "cancelled" => ("Deploy cancelled", format!("{} was cancelled.", env)),
        _           => ("Deploy finished",  format!("{}: {}", env, status)),
    };

    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!("notification failed: {}", e);
    }
}

fn trim_eol(s: &mut String) {
    if s.ends_with('\n') { s.pop(); }
    if s.ends_with('\r') { s.pop(); }
}

fn shell_quote(s: &str) -> String {
    let escaped = s.replace('\'', "'\\''");
    format!("'{}'", escaped)
}
