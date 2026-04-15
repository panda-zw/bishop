use crate::types::Environment;
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

/// Remote file format matches the ./deploy script:
///   user@host:app-name.domain
/// where app-name is everything up to the first '.', domain is the rest.
/// If there's no '.', domain is empty.
pub fn parse_remote_file(env_name: &str, file_path: &Path) -> Result<Environment> {
    let raw = fs::read_to_string(file_path)?;
    let raw: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if raw.is_empty() {
        return Err(anyhow!("remote file {} is empty", env_name));
    }

    let (ssh_part, app_part) = raw
        .split_once(':')
        .ok_or_else(|| anyhow!("remote '{}' missing ':' separator", env_name))?;

    let (ssh_user, ssh_host) = ssh_part
        .split_once('@')
        .ok_or_else(|| anyhow!("remote '{}' ssh part missing '@'", env_name))?;

    let (app_name, domain) = match app_part.split_once('.') {
        Some((name, dom)) => (name.to_string(), Some(dom.to_string())),
        None => (app_part.to_string(), None),
    };

    let app_dir = format!("/opt/apps/{}", app_name);

    Ok(Environment {
        name: env_name.to_string(),
        ssh_user: ssh_user.to_string(),
        ssh_host: ssh_host.to_string(),
        app_name,
        domain,
        app_dir,
    })
}

pub fn list_remotes(project_path: &Path) -> Result<Vec<Environment>> {
    let dir = project_path.join(".deploy").join("remotes");
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut envs = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) if !n.starts_with('.') => n.to_string(),
            _ => continue,
        };
        match parse_remote_file(&name, &path) {
            Ok(env) => envs.push(env),
            Err(e) => tracing::warn!("skipping remote {}: {}", name, e),
        }
    }
    envs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(envs)
}
