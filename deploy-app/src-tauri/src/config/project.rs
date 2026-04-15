use crate::config::remote;
use crate::types::Project;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

pub fn load_project(path: &Path) -> Result<Project> {
    let path = path.canonicalize()?;
    let deploy_dir = path.join(".deploy");
    if !deploy_dir.is_dir() {
        return Err(anyhow!(
            "no .deploy/ directory at {}",
            path.display()
        ));
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project")
        .to_string();

    let remotes = remote::list_remotes(&path)?;
    let git_repo = read_git_repo(&path);

    Ok(Project {
        path: path.to_string_lossy().to_string(),
        name,
        git_repo,
        remotes,
    })
}

/// Parse a git config file and return the remote in `<owner>/<repo>` form.
/// Handles the three common url shapes:
///   - ssh     : `git@github.com:owner/repo(.git)?`
///   - https   : `https://github.com/owner/repo(.git)?`  (optionally with `user:pass@`)
///   - git     : `git://github.com/owner/repo(.git)?`
fn read_git_repo(path: &PathBuf) -> Option<String> {
    let content = std::fs::read_to_string(path.join(".git").join("config")).ok()?;
    for line in content.lines() {
        let t = line.trim();
        let Some(raw) = t.strip_prefix("url = ") else { continue };
        let url = raw.trim().trim_end_matches('/').trim_end_matches(".git");
        return Some(normalize_git_url(url));
    }
    None
}

fn normalize_git_url(url: &str) -> String {
    // scp-style SSH: `git@host:owner/repo`
    // (only treat the first `:` as the separator if there's an `@` before it)
    if let (Some(at), Some(colon)) = (url.find('@'), url.find(':')) {
        if at < colon {
            return url[colon + 1..].to_string();
        }
    }

    // http(s) / git protocols — strip the scheme, then strip the host, then
    // whatever remains is the path (owner/repo or owner/group/repo).
    for scheme in ["https://", "http://", "git://", "ssh://"] {
        if let Some(rest) = url.strip_prefix(scheme) {
            let no_auth = rest.rsplit_once('@').map(|(_, r)| r).unwrap_or(rest);
            if let Some((_host, path)) = no_auth.split_once('/') {
                return path.to_string();
            }
            return no_auth.to_string();
        }
    }

    // Unknown shape — return as-is.
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::normalize_git_url;

    #[test] fn ssh_shape() {
        assert_eq!(normalize_git_url("git@github.com:acme/sample-app"), "acme/sample-app");
    }
    #[test] fn https_shape() {
        assert_eq!(normalize_git_url("https://github.com/acme/sample-app"), "acme/sample-app");
    }
    #[test] fn https_with_auth() {
        assert_eq!(normalize_git_url("https://token@github.com/acme/sample-app"), "acme/sample-app");
    }
    #[test] fn nested_group() {
        assert_eq!(normalize_git_url("https://gitlab.com/acme/group/subgroup/project"), "acme/group/subgroup/project");
    }
    #[test] fn ssh_nested() {
        assert_eq!(normalize_git_url("git@gitlab.com:acme/subgroup/project"), "acme/subgroup/project");
    }
}
