use crate::config::project::load_project;
use crate::ssh;
use crate::types::Container;
use serde::Deserialize;
use std::path::PathBuf;

fn err(e: anyhow::Error) -> String { format!("{:#}", e) }

#[derive(Debug, Deserialize)]
struct DockerPsRow {
    #[serde(rename = "Names")]    names: String,
    #[serde(rename = "State")]    state: String,
    #[serde(rename = "Status")]   status: String,
    #[serde(rename = "Image")]    image: String,
    #[serde(rename = "Ports", default)] ports: String,
}

/// Returns containers whose name starts with `<app_name>-`, matching the ./deploy convention.
#[tauri::command]
pub async fn get_containers(project_path: String, env: String) -> Result<Vec<Container>, String> {
    let proj = load_project(&PathBuf::from(&project_path)).map_err(err)?;
    let env = proj.remotes.into_iter().find(|e| e.name == env)
        .ok_or_else(|| format!("environment '{}' not found", env))?;

    // docker ps emits one JSON object per line (not a JSON array).
    let cmd = "docker ps -a --no-trunc --format '{{json .}}'";
    let stdout = ssh::exec(&env.ssh_user, &env.ssh_host, cmd).await
        .map_err(err)?;

    let prefix = format!("{}-", env.app_name);
    let mut out = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let row: DockerPsRow = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => { tracing::warn!("docker ps row parse error: {} ({})", e, line); continue; }
        };
        if !row.names.starts_with(&prefix) && row.names != env.app_name {
            continue;
        }
        let health = extract_health(&row.status);
        let ports = row.ports.split(',').map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()).collect();
        out.push(Container {
            name: row.names,
            state: row.state,
            status: row.status,
            image: row.image,
            ports,
            health,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn extract_health(status: &str) -> Option<String> {
    // docker "Status" strings include "(healthy)", "(unhealthy)", "(health: starting)"
    let s = status.to_lowercase();
    if s.contains("(healthy)") { Some("healthy".into()) }
    else if s.contains("(unhealthy)") { Some("unhealthy".into()) }
    else if s.contains("health: starting") { Some("starting".into()) }
    else { None }
}
