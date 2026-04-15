use crate::config::{project, store};
use crate::ssh;
use crate::types::Project;
use std::path::PathBuf;

fn err(e: anyhow::Error) -> String { format!("{:#}", e) }

#[tauri::command]
pub fn list_projects() -> Result<Vec<Project>, String> {
    let settings = store::load();
    let mut out = Vec::new();
    for p in &settings.project_paths {
        match project::load_project(&PathBuf::from(p)) {
            Ok(proj) => out.push(proj),
            Err(e) => tracing::warn!("skipping stored project {}: {}", p, e),
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn add_project(path: String) -> Result<Project, String> {
    let proj = project::load_project(&PathBuf::from(&path)).map_err(err)?;
    let mut settings = store::load();
    if !settings.project_paths.iter().any(|p| p == &proj.path) {
        settings.project_paths.push(proj.path.clone());
        store::save(&settings).map_err(err)?;
    }
    Ok(proj)
}

#[tauri::command]
pub fn remove_project(path: String) -> Result<(), String> {
    let mut settings = store::load();
    settings.project_paths.retain(|p| p != &path);
    store::save(&settings).map_err(err)
}

#[tauri::command]
pub async fn ping_env(project_path: String, env: String) -> Result<bool, String> {
    let proj = project::load_project(&PathBuf::from(&project_path)).map_err(err)?;
    let environment = proj.remotes.into_iter().find(|e| e.name == env)
        .ok_or_else(|| format!("environment '{}' not found", env))?;
    Ok(ssh::reachable(&environment.ssh_user, &environment.ssh_host).await)
}
