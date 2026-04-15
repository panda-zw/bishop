//! SSH connectivity probe used by the onboarding wizard before it runs the
//! slow setup-server command — failing here is cheap (seconds) and lets the
//! user fix their ~/.ssh/config, DNS, or firewall before burning 3–5 minutes.
//!
//! The structured result shape matches `BishopError` on the failure path so
//! the frontend can render the same ErrorBanner it uses for deploys.

use crate::errors::{match_error, BishopError};
use crate::ssh;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTestResult {
    pub ok: bool,
    /// Free-form success message, e.g. the output of `uname -a` on the host.
    pub message: Option<String>,
    /// Structured failure when we recognize the error shape.
    pub error: Option<BishopError>,
    /// Raw stderr or exception text — always present on failure for power users.
    pub raw: Option<String>,
}

#[tauri::command]
pub async fn test_ssh(user: String, host: String) -> Result<SshTestResult, String> {
    // `uname -a` is cheap, ubiquitous, and gives the frontend something to
    // show ("Connected to Linux ip-… 5.15.0-… x86_64") as positive feedback.
    match ssh::exec(&user, &host, "uname -a").await {
        Ok(out) => Ok(SshTestResult {
            ok: true,
            message: Some(out.trim().to_string()),
            error: None,
            raw: None,
        }),
        Err(e) => {
            let raw = format!("{}", e);
            Ok(SshTestResult {
                ok: false,
                message: None,
                error: match_error(&raw),
                raw: Some(raw),
            })
        }
    }
}
