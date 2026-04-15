//! Resolves which `./deploy` script to invoke for a given project.
//!
//! Preference order:
//!   1. `<project>/deploy` if present (project-local copy — wins so repos can
//!      pin their CLI version independently of Bishop).
//!   2. Bundled resource `resources/deploy` shipped with the app — used as a
//!      fallback so a freshly-inited project works out of the box.

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub fn bundled_path(app: &AppHandle) -> Result<PathBuf> {
    let resource_dir = app.path().resource_dir()
        .map_err(|e| anyhow!("no resource dir: {}", e))?;
    let path = resource_dir.join("resources").join("deploy");
    if !path.exists() {
        return Err(anyhow!("bundled deploy script missing at {}", path.display()));
    }
    Ok(path)
}

pub fn resolve(app: &AppHandle, project_dir: &Path) -> Result<PathBuf> {
    let local = project_dir.join("deploy");
    if local.exists() {
        return Ok(local);
    }
    bundled_path(app)
}

/// Copy the bundled deploy script into the project root and mark it executable.
/// Overwrites if a file is already present at the target path.
#[tauri::command]
pub fn install_deploy_script(app: AppHandle, project_path: String) -> Result<String, String> {
    let project_dir = PathBuf::from(&project_path);
    if !project_dir.is_dir() {
        return Err(format!("not a directory: {}", project_dir.display()));
    }
    let src = bundled_path(&app).map_err(|e| format!("{:#}", e))?;
    let dst = project_dir.join("deploy");
    std::fs::copy(&src, &dst).map_err(|e| format!("copy failed: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dst)
            .map_err(|e| format!("stat failed: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dst, perms)
            .map_err(|e| format!("chmod failed: {}", e))?;
    }

    Ok(dst.to_string_lossy().to_string())
}

/// Reports whether the project has a local deploy script.
#[tauri::command]
pub fn has_local_deploy_script(project_path: String) -> bool {
    PathBuf::from(&project_path).join("deploy").is_file()
}
