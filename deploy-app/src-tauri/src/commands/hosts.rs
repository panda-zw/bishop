//! Host inventory commands:
//!   - Saved ad-hoc hosts (persisted in Bishop's settings.json)
//!   - ~/.ssh/config Host aliases (read-only, surfaced for convenience)

use crate::config::store::{self, SavedHost};
use serde::Serialize;
use std::fs;
use uuid::Uuid;

fn err(e: anyhow::Error) -> String { format!("{:#}", e) }

#[tauri::command]
pub fn list_saved_hosts() -> Vec<SavedHost> {
    store::load().saved_hosts
}

#[tauri::command]
pub fn add_saved_host(mut host: SavedHost) -> Result<SavedHost, String> {
    let mut settings = store::load();
    if host.id.is_empty() { host.id = Uuid::new_v4().to_string(); }
    settings.saved_hosts.push(host.clone());
    store::save(&settings).map_err(err)?;
    Ok(host)
}

#[tauri::command]
pub fn update_saved_host(host: SavedHost) -> Result<(), String> {
    let mut settings = store::load();
    let Some(idx) = settings.saved_hosts.iter().position(|h| h.id == host.id) else {
        return Err(format!("host '{}' not found", host.id));
    };
    settings.saved_hosts[idx] = host;
    store::save(&settings).map_err(err)
}

#[tauri::command]
pub fn remove_saved_host(id: String) -> Result<(), String> {
    let mut settings = store::load();
    settings.saved_hosts.retain(|h| h.id != id);
    store::save(&settings).map_err(err)
}

#[derive(Debug, Serialize, Clone)]
pub struct SshConfigHost {
    pub alias: String,
    pub user: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
}

#[tauri::command]
pub fn list_ssh_config_hosts() -> Vec<SshConfigHost> {
    let Some(home) = dirs::home_dir() else { return Vec::new() };
    let path = home.join(".ssh").join("config");
    let Ok(content) = fs::read_to_string(&path) else { return Vec::new() };
    parse_ssh_config(&content)
}

/// Minimal ssh_config parser: walks `Host <patterns>` blocks and collects a few
/// keys we care about. Skips any alias containing '*' or '?' (wildcards).
fn parse_ssh_config(content: &str) -> Vec<SshConfigHost> {
    let mut out: Vec<SshConfigHost> = Vec::new();
    let mut current: Vec<SshConfigHost> = Vec::new();

    for raw in content.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() { continue; }

        let mut parts = line.splitn(2, |c: char| c == '=' || c.is_whitespace());
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim_start_matches(['=', ' ']).trim();

        if key.eq_ignore_ascii_case("Host") {
            out.extend(current.drain(..));
            for alias in value.split_whitespace() {
                if alias.contains('*') || alias.contains('?') { continue; }
                current.push(SshConfigHost {
                    alias: alias.to_string(),
                    user: None, hostname: None, port: None,
                    identity_file: None, proxy_jump: None,
                });
            }
        } else if !current.is_empty() {
            for h in current.iter_mut() {
                match key.to_ascii_lowercase().as_str() {
                    "user"         => h.user = Some(value.to_string()),
                    "hostname"     => h.hostname = Some(value.to_string()),
                    "port"         => h.port = value.parse().ok(),
                    "identityfile" => h.identity_file = Some(value.to_string()),
                    "proxyjump"    => h.proxy_jump = Some(value.to_string()),
                    _ => {}
                }
            }
        }
    }
    out.extend(current.drain(..));

    // Collapse duplicates — ssh_config allows the same alias in multiple
    // Host blocks. Keep the first occurrence of each alias.
    let mut seen = std::collections::HashSet::new();
    out.retain(|h| seen.insert(h.alias.clone()));
    out
}
