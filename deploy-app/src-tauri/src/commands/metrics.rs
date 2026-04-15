//! `docker stats` streaming for live CPU/memory of app containers.
//! One blocking ssh process per env, streaming `--format json --no-trunc`
//! lines as Tauri events.

use crate::config::project::load_project;
use crate::ssh;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncBufReadExt;
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Default)]
pub struct MetricsStreams {
    inner: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
}

#[tauri::command]
pub async fn start_metrics(
    app: AppHandle,
    streams: State<'_, MetricsStreams>,
    project_path: String,
    env: String,
) -> Result<String, String> {
    let proj = load_project(&PathBuf::from(&project_path)).map_err(|e| format!("{:#}", e))?;
    let environment = proj.remotes.into_iter().find(|e| e.name == env)
        .ok_or_else(|| format!("environment '{}' not found", env))?;

    let stream_id = Uuid::new_v4().to_string();
    let event = format!("metrics:{}", stream_id);

    let (cancel_tx, mut cancel_rx) = oneshot::channel::<()>();
    streams.inner.lock().insert(stream_id.clone(), cancel_tx);

    let user = environment.ssh_user.clone();
    let host = environment.ssh_host.clone();
    // docker stats streams one JSON object per line per container.
    let cmd = "docker stats --no-trunc --format '{{json .}}'";

    let app_handle = app.clone();
    tokio::spawn(async move {
        let streaming = match ssh::spawn_stream(&user, &host, cmd) {
            Ok(s) => s,
            Err(e) => { let _ = app_handle.emit(&event, format!("[error: {}]", e)); return; }
        };
        let mut child = streaming.child;
        let mut stdout = streaming.stdout;

        let app_out = app_handle.clone();
        let event_out = event.clone();
        let task = tokio::spawn(async move {
            loop {
                let mut line = String::new();
                match stdout.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() && trimmed.starts_with('{') {
                            let _ = app_out.emit(&event_out, trimmed.to_string());
                        }
                    }
                }
            }
        });

        tokio::select! {
            _ = &mut cancel_rx => { let _ = child.start_kill(); }
            _ = child.wait() => {}
        }
        task.abort();
    });

    Ok(stream_id)
}

#[tauri::command]
pub async fn stop_metrics(streams: State<'_, MetricsStreams>, stream_id: String) -> Result<(), String> {
    if let Some(tx) = streams.inner.lock().remove(&stream_id) { let _ = tx.send(()); }
    Ok(())
}
