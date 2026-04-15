//! Thin wrapper over the system `ssh` binary.
//!
//! Why not russh: the system ssh honors `~/.ssh/config` (ProxyJump, host aliases,
//! IdentityFile, ssh-agent) with no extra work, and matches the CLI's auth surface
//! exactly. We enable ControlMaster multiplexing so repeated commands reuse a single
//! TCP+auth handshake per host.
//!
//! ControlPath is scoped to a per-run tmpdir so multiple app instances don't collide.

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::BufReader;
use tokio::process::{Child, Command};

/// ControlPath must stay well under 104 bytes on macOS (Unix socket limit).
/// `~/.ssh/bishop/` is short + stable; we prepend nothing else so total path
/// stays ~50 chars even with long usernames.
fn control_dir() -> PathBuf {
    crate::paths::ssh_control_dir()
}

fn base_args(user: &str, host: &str) -> Vec<String> {
    let cp = control_dir().join("%C").to_string_lossy().to_string();
    vec![
        "-o".into(), "BatchMode=yes".into(),
        "-o".into(), "ConnectTimeout=10".into(),
        "-o".into(), "ServerAliveInterval=30".into(),
        "-o".into(), "ControlMaster=auto".into(),
        "-o".into(), format!("ControlPath={}", cp),
        "-o".into(), "ControlPersist=600".into(),
        format!("{}@{}", user, host),
    ]
}

/// Run a one-shot command over SSH and return stdout as a String.
/// Stderr is captured and mapped to a friendlier error message on non-zero exit.
pub async fn exec(user: &str, host: &str, command: &str) -> Result<String> {
    let mut args = base_args(user, host);
    args.push(command.to_string());

    let output = Command::new("ssh").args(&args).output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("{}", friendly_ssh_error(user, host, &stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Fast reachability probe — used by the sidebar status dots.
pub async fn reachable(user: &str, host: &str) -> bool {
    let cp = control_dir().join("%C").to_string_lossy().to_string();
    let args: Vec<String> = vec![
        "-o".into(), "BatchMode=yes".into(),
        "-o".into(), "ConnectTimeout=4".into(),
        "-o".into(), "ControlMaster=auto".into(),
        "-o".into(), format!("ControlPath={}", cp),
        "-o".into(), "ControlPersist=600".into(),
        format!("{}@{}", user, host),
        "echo ok".into(),
    ];
    match Command::new("ssh").args(&args).output().await {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

fn friendly_ssh_error(user: &str, host: &str, stderr: &str) -> String {
    let s = stderr.to_lowercase();
    let target = format!("{}@{}", user, host);
    if s.contains("permission denied") {
        format!("SSH permission denied for {target} — is your key added (ssh-add) or in ~/.ssh/config?")
    } else if s.contains("connection refused") {
        format!("Connection refused by {target} — is SSH running on port 22?")
    } else if s.contains("connection timed out") || s.contains("operation timed out") {
        format!("Connection timed out to {target} — host unreachable or firewalled")
    } else if s.contains("host is down") || s.contains("no route to host") {
        format!("Host {target} is unreachable")
    } else if s.contains("could not resolve hostname") || s.contains("name or service not known") {
        format!("Cannot resolve hostname in {target}")
    } else if s.contains("host key verification failed") {
        format!("Host key mismatch for {target} — check ~/.ssh/known_hosts")
    } else {
        format!("ssh {target} failed: {}", stderr.trim())
    }
}

/// Spawn a long-running SSH command and return the Child plus a line-stream.
pub fn spawn_stream(user: &str, host: &str, command: &str) -> Result<StreamingChild> {
    let mut args = base_args(user, host);
    args.push(command.to_string());

    let mut child = Command::new("ssh")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;

    let stdout = child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?;
    let stderr = child.stderr.take().ok_or_else(|| anyhow!("no stderr"))?;

    Ok(StreamingChild {
        child,
        stdout: BufReader::new(stdout),
        stderr: BufReader::new(stderr),
    })
}

pub struct StreamingChild {
    pub child: Child,
    pub stdout: BufReader<tokio::process::ChildStdout>,
    pub stderr: BufReader<tokio::process::ChildStderr>,
}
