//! Per-host scrollback persistence. Raw UTF-8 bytes (with ANSI escapes) saved
//! under ~/Library/Application Support/overlord/sessions/<key>.txt.
//!
//! Keys are opaque to the backend — the frontend picks a stable identifier
//! (e.g. "saved:<uuid>", "alias:<alias>", "user@host") and we sanitize it
//! for filesystem safety.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

fn sessions_dir() -> Result<PathBuf, String> {
    let base = crate::paths::config_dir()
        .map_err(|e| format!("{:#}", e))?
        .join("sessions");
    fs::create_dir_all(&base).map_err(|e| format!("mkdir: {}", e))?;
    Ok(base)
}

/// Keep filenames safe: lowercase, alnum + `-_.@` only, collapse anything else.
fn sanitize(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for c in key.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '@') {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.len() > 160 { out.truncate(160); }
    out
}

fn session_file(key: &str) -> Result<PathBuf, String> {
    Ok(sessions_dir()?.join(format!("{}.txt", sanitize(key))))
}

#[derive(Debug, Serialize)]
pub struct Scrollback {
    /// Raw bytes of the scrollback, base64-encoded. Empty string if none.
    pub data: String,
    /// Unix seconds of last save, or 0 if no prior session.
    pub saved_at: u64,
}

#[tauri::command]
pub fn read_scrollback(host_key: String) -> Result<Scrollback, String> {
    let path = session_file(&host_key)?;
    if !path.exists() {
        return Ok(Scrollback { data: String::new(), saved_at: 0 });
    }
    let bytes = fs::read(&path).map_err(|e| format!("read: {}", e))?;
    let saved_at = fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(Scrollback { data: B64.encode(&bytes), saved_at })
}

#[tauri::command]
pub fn write_scrollback(host_key: String, data: String) -> Result<(), String> {
    let bytes = B64.decode(data.as_bytes()).map_err(|e| format!("bad base64: {}", e))?;
    let path = session_file(&host_key)?;
    // Atomic-ish write: write to tmp, rename.
    let tmp = path.with_extension("txt.tmp");
    fs::write(&tmp, &bytes).map_err(|e| format!("write: {}", e))?;
    fs::rename(&tmp, &path).map_err(|e| format!("rename: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn clear_scrollback(host_key: String) -> Result<(), String> {
    let path = session_file(&host_key)?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("remove: {}", e))?;
    }
    Ok(())
}
