//! "Use a sample app" clone helper used by the onboarding wizard.
//!
//! Shells out to the system `git` binary so we inherit the user's SSH keys,
//! credential helpers, and `~/.gitconfig`. Using a Rust git library would
//! duplicate all that for no win.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

/// The curated Bishop starter. Publicly cloneable over HTTPS so no auth is
/// required. If we add more starters (Rails, Django, Go, etc.) this becomes
/// a lookup keyed by `kind`.
const STARTER_URL: &str = "https://github.com/panda-zw/bishop-starter-nextjs.git";

#[tauri::command]
pub async fn clone_sample(dest_parent: String, folder_name: String) -> Result<String, String> {
    if folder_name.trim().is_empty() {
        return Err("folder name is required".into());
    }
    // Guard against path traversal / sneaky folder names — we write under a
    // user-picked parent, and the folder_name is a UI-editable field.
    if folder_name.contains('/') || folder_name.contains('\\') || folder_name.starts_with('.') {
        return Err("folder name must be a simple filename (no slashes or leading dot)".into());
    }

    let parent = PathBuf::from(&dest_parent)
        .canonicalize()
        .map_err(|e| format!("invalid destination parent: {}", e))?;
    if !parent.is_dir() {
        return Err("destination parent is not a directory".into());
    }

    let dest: PathBuf = parent.join(&folder_name);
    if dest.exists() {
        return Err(format!(
            "{} already exists — pick a different folder name or remove the existing one",
            dest.display()
        ));
    }

    run_git(&["clone", "--depth", "1", STARTER_URL, dest.to_str().unwrap()]).await?;

    // Drop the starter's git history — the user's project shouldn't inherit
    // the starter's commits. Their first `git init` starts fresh.
    remove_git_dir(&dest)?;

    Ok(dest.to_string_lossy().to_string())
}

async fn run_git(args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("failed to spawn git: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git {}: {}", args.join(" "), stderr.trim()));
    }
    Ok(())
}

fn remove_git_dir(project: &Path) -> Result<(), String> {
    let git_dir = project.join(".git");
    if git_dir.is_dir() {
        std::fs::remove_dir_all(&git_dir).map_err(|e| format!("remove .git: {}", e))?;
    }
    Ok(())
}
