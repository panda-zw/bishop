//! Read/write the two files Bishop can scaffold for a project:
//!   - `.deploy/infra/docker-compose.prod.yml` — the "remote" compose file the
//!     deploy script scp's to the server. Despite the "prod" suffix, it's used
//!     for every environment (staging, production, preview).
//!   - `Dockerfile` at the project root.
//!
//! We don't invent new locations — we write where the CLI expects to find them.

use std::fs;
use std::path::{Path, PathBuf};

fn err<E: std::fmt::Display>(e: E) -> String { format!("{}", e) }

/// Canonicalize the caller-supplied project path and confirm it's an existing
/// directory. This turns a renderer-supplied path into an absolute, symlink-
/// resolved root we can safely join scaffold-relative paths onto — blocking
/// "write arbitrary file anywhere" via a malicious `project_path`.
fn canonical_project(project_path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(project_path)
        .canonicalize()
        .map_err(|e| format!("invalid project path: {}", e))?;
    if !p.is_dir() {
        return Err("project path is not a directory".into());
    }
    Ok(p)
}

/// Lenient variant for read/has checks: returns None if the project path
/// doesn't exist yet, so the UI can render "not present" without erroring.
fn try_canonical_project(project_path: &str) -> Option<PathBuf> {
    PathBuf::from(project_path).canonicalize().ok().filter(|p| p.is_dir())
}

fn compose_rel() -> &'static Path { Path::new(".deploy/infra/docker-compose.prod.yml") }
fn dockerfile_rel() -> &'static Path { Path::new("Dockerfile") }
fn shared_compose_rel() -> &'static Path { Path::new(".deploy/infra/shared/docker-compose.yml") }
fn shared_init_db_rel() -> &'static Path { Path::new(".deploy/infra/shared/init-databases.sh") }
fn shared_traefik_rel() -> &'static Path { Path::new(".deploy/infra/shared/traefik/traefik.yml") }
fn dockerignore_rel() -> &'static Path { Path::new(".dockerignore") }

#[tauri::command]
pub fn has_compose_file(project_path: String) -> bool {
    try_canonical_project(&project_path)
        .map(|root| root.join(compose_rel()).is_file())
        .unwrap_or(false)
}

#[tauri::command]
pub fn read_compose_file(project_path: String) -> Result<Option<String>, String> {
    let root = canonical_project(&project_path)?;
    let p = root.join(compose_rel());
    if !p.is_file() { return Ok(None); }
    fs::read_to_string(&p).map(Some).map_err(err)
}

#[tauri::command]
pub fn write_compose_file(project_path: String, content: String) -> Result<(), String> {
    let root = canonical_project(&project_path)?;
    let p = root.join(compose_rel());
    if let Some(parent) = p.parent() { fs::create_dir_all(parent).map_err(err)?; }
    fs::write(&p, content).map_err(err)
}

#[tauri::command]
pub fn has_dockerfile(project_path: String) -> bool {
    try_canonical_project(&project_path)
        .map(|root| root.join(dockerfile_rel()).is_file())
        .unwrap_or(false)
}

#[tauri::command]
pub fn read_dockerfile(project_path: String) -> Result<Option<String>, String> {
    let root = canonical_project(&project_path)?;
    let p = root.join(dockerfile_rel());
    if !p.is_file() { return Ok(None); }
    fs::read_to_string(&p).map(Some).map_err(err)
}

#[tauri::command]
pub fn write_dockerfile(project_path: String, content: String) -> Result<(), String> {
    let root = canonical_project(&project_path)?;
    fs::write(root.join(dockerfile_rel()), content).map_err(err)
}

// -------------------------------------------------------------------------
// Shared infrastructure files (Traefik / shared Postgres / shared Redis).
// These live at .deploy/infra/shared/* and are scp'd to /opt/shared/ on the
// server by both `setup-server` and every deploy.
// -------------------------------------------------------------------------

#[tauri::command]
pub fn has_shared_files(project_path: String) -> bool {
    let Some(root) = try_canonical_project(&project_path) else { return false; };
    root.join(shared_compose_rel()).is_file()
        && root.join(shared_traefik_rel()).is_file()
        && root.join(shared_init_db_rel()).is_file()
}

#[derive(serde::Serialize)]
pub struct SharedStatus {
    compose: bool,
    init_db: bool,
    traefik: bool,
}

#[tauri::command]
pub fn shared_file_status(project_path: String) -> SharedStatus {
    let Some(root) = try_canonical_project(&project_path) else {
        return SharedStatus { compose: false, init_db: false, traefik: false };
    };
    SharedStatus {
        compose: root.join(shared_compose_rel()).is_file(),
        init_db: root.join(shared_init_db_rel()).is_file(),
        traefik: root.join(shared_traefik_rel()).is_file(),
    }
}

#[tauri::command]
pub fn read_shared_compose(project_path: String) -> Result<Option<String>, String> {
    let root = canonical_project(&project_path)?;
    let p = root.join(shared_compose_rel());
    if !p.is_file() { return Ok(None); }
    fs::read_to_string(&p).map(Some).map_err(err)
}

#[tauri::command]
pub fn write_shared_compose(project_path: String, content: String) -> Result<(), String> {
    let root = canonical_project(&project_path)?;
    let p = root.join(shared_compose_rel());
    if let Some(parent) = p.parent() { fs::create_dir_all(parent).map_err(err)?; }
    fs::write(&p, content).map_err(err)
}

#[tauri::command]
pub fn read_shared_traefik(project_path: String) -> Result<Option<String>, String> {
    let root = canonical_project(&project_path)?;
    let p = root.join(shared_traefik_rel());
    if !p.is_file() { return Ok(None); }
    fs::read_to_string(&p).map(Some).map_err(err)
}

#[tauri::command]
pub fn write_shared_traefik(project_path: String, content: String) -> Result<(), String> {
    let root = canonical_project(&project_path)?;
    let p = root.join(shared_traefik_rel());
    if let Some(parent) = p.parent() { fs::create_dir_all(parent).map_err(err)?; }
    fs::write(&p, content).map_err(err)
}

#[tauri::command]
pub fn read_shared_init_db(project_path: String) -> Result<Option<String>, String> {
    let root = canonical_project(&project_path)?;
    let p = root.join(shared_init_db_rel());
    if !p.is_file() { return Ok(None); }
    fs::read_to_string(&p).map(Some).map_err(err)
}

#[tauri::command]
pub fn write_shared_init_db(project_path: String, content: String) -> Result<(), String> {
    let root = canonical_project(&project_path)?;
    let p = root.join(shared_init_db_rel());
    if let Some(parent) = p.parent() { fs::create_dir_all(parent).map_err(err)?; }
    fs::write(&p, content).map_err(err)?;

    // Make sure the script is executable so `setup-server` can chmod-preserve it.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&p).map_err(err)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&p, perms).map_err(err)?;
    }
    Ok(())
}

// -------------------------------------------------------------------------
// .dockerignore (project root)
// -------------------------------------------------------------------------

#[tauri::command]
pub fn has_dockerignore(project_path: String) -> bool {
    try_canonical_project(&project_path)
        .map(|root| root.join(dockerignore_rel()).is_file())
        .unwrap_or(false)
}

#[tauri::command]
pub fn read_dockerignore(project_path: String) -> Result<Option<String>, String> {
    let root = canonical_project(&project_path)?;
    let p = root.join(dockerignore_rel());
    if !p.is_file() { return Ok(None); }
    fs::read_to_string(&p).map(Some).map_err(err)
}

#[tauri::command]
pub fn write_dockerignore(project_path: String, content: String) -> Result<(), String> {
    let root = canonical_project(&project_path)?;
    fs::write(root.join(dockerignore_rel()), content).map_err(err)
}
