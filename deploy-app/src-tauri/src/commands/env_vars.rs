//! Read-only .env viewer. Fetches `<app_dir>/.env` over SSH, parses into
//! structured entries, and flags secret keys (same pattern as the CLI:
//! SECRET|PASSWORD|KEY|TOKEN). Masking happens in the frontend — we return
//! full values plus an `is_secret` flag.

use crate::config::project::load_project;
use crate::ssh;
use crate::types::Environment;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum EnvLine {
    Comment { text: String },
    Blank,
    Var { key: String, value: String, is_secret: bool },
    Raw { text: String },
}

#[tauri::command]
pub async fn get_env_vars(project_path: String, env: String) -> Result<Vec<EnvLine>, String> {
    let proj = load_project(&PathBuf::from(&project_path)).map_err(|e| format!("{:#}", e))?;
    let environment = proj.remotes.into_iter().find(|e| e.name == env)
        .ok_or_else(|| format!("environment '{}' not found", env))?;

    let cmd = format!("cat {}/.env", shell_quote(&environment.app_dir));
    let stdout = ssh::exec(&environment.ssh_user, &environment.ssh_host, &cmd).await
        .map_err(|e| format!("{:#}", e))?;

    Ok(parse_env(&stdout))
}

fn parse_env(content: &str) -> Vec<EnvLine> {
    content.lines().map(|line| {
        let trimmed = line.trim_end();
        if trimmed.is_empty() { return EnvLine::Blank; }
        if trimmed.starts_with('#') { return EnvLine::Comment { text: trimmed.to_string() }; }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            if is_valid_key(key) {
                return EnvLine::Var {
                    key: key.to_string(),
                    value: value.to_string(),
                    is_secret: is_secret_key(key),
                };
            }
        }
        EnvLine::Raw { text: trimmed.to_string() }
    }).collect()
}

fn is_valid_key(k: &str) -> bool {
    !k.is_empty() && k.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn is_secret_key(k: &str) -> bool {
    let u = k.to_uppercase();
    u.contains("SECRET") || u.contains("PASSWORD") || u.contains("KEY") || u.contains("TOKEN")
}

fn shell_quote(s: &str) -> String {
    let escaped = s.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

/// Validate key: uppercase letters, digits, underscores, must not start with digit.
fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("key is empty".into());
    }
    if key.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return Err("key must not start with a digit".into());
    }
    if !key.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') {
        return Err("key must be uppercase letters, digits, or underscores".into());
    }
    Ok(())
}

async fn read_env_file(env: &Environment) -> Result<String, anyhow::Error> {
    let cmd = format!("cat {}/.env", shell_quote(&env.app_dir));
    ssh::exec(&env.ssh_user, &env.ssh_host, &cmd).await
}

/// Overwrite `<app_dir>/.env` atomically by sending the content base64-encoded.
/// Avoids shell-escaping entirely.
async fn write_env_file(env: &Environment, content: &str) -> Result<(), anyhow::Error> {
    let encoded = B64.encode(content.as_bytes());
    let app_dir = shell_quote(&env.app_dir);
    let remote_cmd = format!(
        "printf '%s' {b64} | base64 -d > {dir}/.env.tmp && mv {dir}/.env.tmp {dir}/.env",
        b64 = shell_quote(&encoded),
        dir = app_dir,
    );
    ssh::exec(&env.ssh_user, &env.ssh_host, &remote_cmd).await.map(|_| ())
}

fn upsert_line(content: &str, key: &str, value: &str) -> String {
    let mut found = false;
    let mut out = String::with_capacity(content.len() + key.len() + value.len() + 2);
    let had_trailing_newline = content.ends_with('\n');

    for (i, line) in content.lines().enumerate() {
        if i > 0 { out.push('\n'); }
        if !found && line_key(line) == Some(key) {
            out.push_str(key);
            out.push('=');
            out.push_str(value);
            found = true;
        } else {
            out.push_str(line);
        }
    }

    if !found {
        if !out.is_empty() { out.push('\n'); }
        out.push_str(key);
        out.push('=');
        out.push_str(value);
        out.push('\n');
    } else if had_trailing_newline {
        out.push('\n');
    }

    out
}

fn delete_line(content: &str, key: &str) -> String {
    let had_trailing_newline = content.ends_with('\n');
    let mut out = String::with_capacity(content.len());
    let mut first = true;
    for line in content.lines() {
        if line_key(line) == Some(key) { continue; }
        if !first { out.push('\n'); }
        out.push_str(line);
        first = false;
    }
    if had_trailing_newline && !out.is_empty() { out.push('\n'); }
    out
}

fn line_key(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') { return None; }
    let (k, _) = trimmed.split_once('=')?;
    let k = k.trim();
    if is_valid_key(k) { Some(k) } else { None }
}

#[tauri::command]
pub async fn set_env_var(
    project_path: String,
    env: String,
    key: String,
    value: String,
) -> Result<(), String> {
    validate_key(&key)?;
    let proj = load_project(&PathBuf::from(&project_path)).map_err(|e| format!("{:#}", e))?;
    let environment = proj.remotes.into_iter().find(|e| e.name == env)
        .ok_or_else(|| format!("environment '{}' not found", env))?;

    let current = read_env_file(&environment).await.map_err(|e| format!("{:#}", e))?;
    let updated = upsert_line(&current, &key, &value);
    write_env_file(&environment, &updated).await.map_err(|e| format!("{:#}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn delete_env_var(
    project_path: String,
    env: String,
    key: String,
) -> Result<(), String> {
    validate_key(&key)?;
    let proj = load_project(&PathBuf::from(&project_path)).map_err(|e| format!("{:#}", e))?;
    let environment = proj.remotes.into_iter().find(|e| e.name == env)
        .ok_or_else(|| format!("environment '{}' not found", env))?;

    let current = read_env_file(&environment).await.map_err(|e| format!("{:#}", e))?;
    let updated = delete_line(&current, &key);
    write_env_file(&environment, &updated).await.map_err(|e| format!("{:#}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_updates_existing() {
        let input = "FOO=1\nBAR=2\n";
        let out = upsert_line(input, "BAR", "99");
        assert_eq!(out, "FOO=1\nBAR=99\n");
    }

    #[test]
    fn upsert_appends_new() {
        let input = "FOO=1\n";
        let out = upsert_line(input, "BAR", "2");
        assert_eq!(out, "FOO=1\nBAR=2\n");
    }

    #[test]
    fn upsert_preserves_comments_and_blanks() {
        let input = "# header\nFOO=1\n\nBAR=2\n";
        let out = upsert_line(input, "FOO", "x");
        assert_eq!(out, "# header\nFOO=x\n\nBAR=2\n");
    }

    #[test]
    fn upsert_handles_value_with_special_chars() {
        let input = "FOO=1\n";
        let out = upsert_line(input, "FOO", "a|b$c 'd\"");
        assert_eq!(out, "FOO=a|b$c 'd\"\n");
    }

    #[test]
    fn delete_removes_line() {
        let input = "FOO=1\nBAR=2\nBAZ=3\n";
        let out = delete_line(input, "BAR");
        assert_eq!(out, "FOO=1\nBAZ=3\n");
    }
}
