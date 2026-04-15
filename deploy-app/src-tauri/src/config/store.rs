use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub project_paths: Vec<String>,
    #[serde(default)]
    pub saved_hosts: Vec<SavedHost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedHost {
    #[serde(default)]
    pub id: String,
    pub label: String,
    pub user: String,
    pub host: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub identity: Option<String>,
    #[serde(default)]
    pub proxy_jump: Option<String>,
    #[serde(default)]
    pub initial_cwd: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    /// If true, the terminal wraps the shell in a named tmux session so it
    /// survives disconnects. Requires tmux installed on the remote.
    #[serde(default)]
    pub use_tmux: bool,
    /// Tmux session name (auto-derived from label if empty). Stable across
    /// reconnects.
    #[serde(default)]
    pub tmux_session: Option<String>,
}

fn settings_path() -> Result<PathBuf> {
    Ok(crate::paths::config_dir()?.join("settings.json"))
}

pub fn load() -> Settings {
    (|| -> Result<Settings> {
        let p = settings_path()?;
        if !p.exists() {
            return Ok(Settings::default());
        }
        let s = fs::read_to_string(p)?;
        Ok(serde_json::from_str(&s)?)
    })()
    .unwrap_or_default()
}

pub fn save(settings: &Settings) -> Result<()> {
    let p = settings_path()?;
    fs::write(p, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}
