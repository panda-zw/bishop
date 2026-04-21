//! Drop/paste → file handlers for the terminal pane.
//!
//! The frontend sends raw bytes + a filename hint; we sanitize the name,
//! stamp it with a timestamp to dodge collisions, and either:
//!   - save under `<config_dir>/pasted-files/` (local tabs), or
//!   - SCP-via-stdin to `/tmp/bishop-pasted/` on the remote host (ssh tabs).
//!
//! The returned path is what the frontend types into the PTY (shell-quoted on
//! the JS side), so the user can immediately reference the file in a command.

use crate::commands::terminal::TerminalTarget;
use crate::paths;
use crate::ssh;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// 200 MB hard cap. Anything larger and the user should scp directly from the
/// terminal rather than round-trip through the webview.
const MAX_BYTES: usize = 200 * 1024 * 1024;

const REMOTE_DIR: &str = "/tmp/bishop-pasted";

/// Decode the base64 payload, enforce the size cap, and return the bytes.
fn decode_and_check(data_b64: &str) -> Result<Vec<u8>, String> {
    let bytes = B64.decode(data_b64.as_bytes())
        .map_err(|e| format!("bad base64: {}", e))?;
    if bytes.len() > MAX_BYTES {
        return Err(format!(
            "file is {:.1} MB — limit is {} MB. Transfer it directly with scp instead.",
            bytes.len() as f64 / 1_048_576.0,
            MAX_BYTES / 1_048_576,
        ));
    }
    Ok(bytes)
}

/// Strip anything that isn't `[A-Za-z0-9._-]` to a single underscore, trim
/// leading dots (so we don't create hidden files), cap at 96 chars, and
/// prefix with a Unix timestamp so two simultaneous pastes don't collide.
fn sanitize_filename(raw: &str) -> String {
    let base = std::path::Path::new(raw)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "pasted".to_string());

    let cleaned: String = base
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '_' })
        .collect();
    let trimmed = cleaned.trim_start_matches('.').trim_start_matches('_');
    let mut final_name = if trimmed.is_empty() { "pasted" } else { trimmed }.to_string();
    if final_name.len() > 96 {
        final_name.truncate(96);
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}-{}", ts, final_name)
}

/// Single-quote a value for safe inclusion in a remote shell command.
fn shell_quote(s: &str) -> String {
    let escaped = s.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

fn local_dir() -> Result<PathBuf, String> {
    let dir = paths::config_dir()
        .map_err(|e| format!("config dir: {}", e))?
        .join("pasted-files");
    fs::create_dir_all(&dir).map_err(|e| format!("create {}: {}", dir.display(), e))?;
    Ok(dir)
}

#[tauri::command]
pub async fn paste_file_local(
    data: String,
    filename: String,
) -> Result<String, String> {
    let bytes = decode_and_check(&data)?;
    let sanitized = sanitize_filename(&filename);
    let dir = local_dir()?;
    let dest = dir.join(&sanitized);
    fs::write(&dest, &bytes).map_err(|e| format!("write {}: {}", dest.display(), e))?;
    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn paste_file_remote(
    target: TerminalTarget,
    data: String,
    filename: String,
) -> Result<String, String> {
    let bytes = decode_and_check(&data)?;
    let sanitized = sanitize_filename(&filename);
    let remote_path = format!("{}/{}", REMOTE_DIR, sanitized);

    // `cat >` is binary-safe and the entire command line is single-quoted,
    // so the sanitized filename (already `[A-Za-z0-9._-]` only) can't escape
    // its quote context.
    let remote_cmd = format!(
        "mkdir -p {} && cat > {}",
        shell_quote(REMOTE_DIR),
        shell_quote(&remote_path),
    );
    let args = target.to_exec_args(&remote_cmd)?;

    ssh::exec_with_stdin(&args, &bytes)
        .await
        .map_err(|e| format!("upload to remote failed: {}", e))?;
    Ok(remote_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_path_and_specials() {
        let s = sanitize_filename("../../../../etc/passwd; rm -rf");
        assert!(!s.contains('/'));
        assert!(!s.contains(';'));
        assert!(!s.contains(".."));
        // Last component preserved after path stripping + special→_ mapping.
        assert!(s.contains("passwd"));
    }

    #[test]
    fn sanitize_preserves_safe_filename() {
        let s = sanitize_filename("screenshot 1.png");
        assert!(s.ends_with("screenshot_1.png"));
    }

    #[test]
    fn sanitize_trims_leading_dots() {
        let s = sanitize_filename(".hidden.txt");
        assert!(!s.contains("-.hidden"));
        assert!(s.contains("hidden.txt"));
    }

    #[test]
    fn sanitize_caps_length() {
        let long = "a".repeat(500);
        let s = sanitize_filename(&long);
        let (_ts, name) = s.split_once('-').unwrap();
        assert_eq!(name.len(), 96);
    }

    #[test]
    fn size_cap_rejects_oversize() {
        let oversize = B64.encode(vec![0u8; MAX_BYTES + 1]);
        assert!(decode_and_check(&oversize).is_err());
    }

    #[test]
    fn size_cap_allows_max() {
        let at_limit = B64.encode(vec![0u8; MAX_BYTES]);
        assert!(decode_and_check(&at_limit).is_ok());
    }
}
