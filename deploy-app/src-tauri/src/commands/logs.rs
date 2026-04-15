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
pub struct LogStreams {
    inner: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
}

#[tauri::command]
pub async fn start_log_stream(
    app: AppHandle,
    streams: State<'_, LogStreams>,
    project_path: String,
    env: String,
    service: String,
) -> Result<String, String> {
    let proj = load_project(&PathBuf::from(&project_path)).map_err(|e| format!("{:#}", e))?;
    let environment = proj.remotes.into_iter().find(|e| e.name == env)
        .ok_or_else(|| format!("environment '{}' not found", env))?;

    let stream_id = Uuid::new_v4().to_string();
    let event = format!("logs:{}", stream_id);

    let user = environment.ssh_user.clone();
    let host = environment.ssh_host.clone();
    let container = shell_quote(&service);
    let cmd = format!("docker logs -f --tail 200 --timestamps {}", container);

    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    streams.inner.lock().insert(stream_id.clone(), cancel_tx);

    let app_handle = app.clone();
    tokio::spawn(async move {
        let streaming = match ssh::spawn_stream(&user, &host, &cmd) {
            Ok(c) => c,
            Err(e) => {
                let _ = app_handle.emit(&event, format!("[error: {}]", e));
                return;
            }
        };

        let mut child = streaming.child;
        let mut stdout = streaming.stdout;
        let mut stderr = streaming.stderr;

        let app_stdout = app_handle.clone();
        let app_stderr = app_handle.clone();
        let event_stdout = event.clone();
        let event_stderr = event.clone();

        let stdout_task = tokio::spawn(async move {
            loop {
                let mut line = String::new();
                match stdout.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        trim_eol(&mut line);
                        let _ = app_stdout.emit(&event_stdout, line);
                    }
                }
            }
        });

        let stderr_task = tokio::spawn(async move {
            loop {
                let mut line = String::new();
                match stderr.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        trim_eol(&mut line);
                        let _ = app_stderr.emit(&event_stderr, format!("[stderr] {}", line));
                    }
                }
            }
        });

        tokio::select! {
            _ = cancel_rx => {
                let _ = child.start_kill();
            }
            _ = child.wait() => {}
        }

        stdout_task.abort();
        stderr_task.abort();
    });

    Ok(stream_id)
}

#[tauri::command]
pub async fn stop_log_stream(streams: State<'_, LogStreams>, stream_id: String) -> Result<(), String> {
    let tx = streams.inner.lock().remove(&stream_id);
    if let Some(tx) = tx {
        let _ = tx.send(());
    }
    Ok(())
}

fn trim_eol(s: &mut String) {
    if s.ends_with('\n') { s.pop(); }
    if s.ends_with('\r') { s.pop(); }
}

fn shell_quote(s: &str) -> String {
    let escaped = s.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

