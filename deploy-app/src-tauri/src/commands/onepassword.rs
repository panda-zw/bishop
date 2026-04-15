//! 1Password integration. Uses the local `op` CLI — the user must be signed in
//! (`op signin`) outside the app. We never store credentials.
//!
//! - `op_status` checks if the CLI is installed and authenticated.
//! - `op_read` resolves a secret reference like `op://Vault/Item/field` to a value.

use tokio::process::Command;

#[derive(serde::Serialize)]
pub struct OpStatus {
    installed: bool,
    signed_in: bool,
}

#[tauri::command]
pub async fn op_status() -> OpStatus {
    let probe = Command::new("op").arg("--version").output().await;
    let installed = probe.as_ref().map(|o| o.status.success()).unwrap_or(false);
    if !installed { return OpStatus { installed: false, signed_in: false }; }

    // `op whoami` exits non-zero if no active session.
    let whoami = Command::new("op").arg("whoami").output().await;
    let signed_in = whoami.map(|o| o.status.success()).unwrap_or(false);

    OpStatus { installed, signed_in }
}

#[tauri::command]
pub async fn op_read(reference: String) -> Result<String, String> {
    if !reference.starts_with("op://") {
        return Err("reference must start with op://".into());
    }
    let out = Command::new("op").arg("read").arg(&reference).output().await
        .map_err(|e| format!("op read: {}", e))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    // op read appends a trailing newline.
    Ok(s.trim_end_matches(['\n', '\r']).to_string())
}
