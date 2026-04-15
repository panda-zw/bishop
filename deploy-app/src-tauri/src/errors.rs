//! Error catalog — turns raw bash/docker/ssh output into structured,
//! actionable `BishopError` payloads the frontend can render.
//!
//! Design: regex-matched, append-only, never matches more than once per text
//! blob (first hit wins). Add entries to `CATALOG` as new error shapes are
//! observed in the wild — they compound in value over time.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

/// Structured error sent to the frontend. The frontend decides how to render
/// `message` + `hint` + `action`; `raw` is kept for power users who want the
/// original text.
#[derive(Debug, Clone, Serialize)]
pub struct BishopError {
    pub code: String,
    pub message: String,
    pub hint: Option<String>,
    pub docs_url: Option<String>,
    pub action: Option<BishopAction>,
    pub raw: Option<String>,
}

/// Optional "Try this" button the frontend can render next to the error.
#[derive(Debug, Clone, Serialize)]
pub struct BishopAction {
    /// Button label, e.g. "Open scaffold", "Retry", "Copy fix command".
    pub label: String,
    /// Discriminator for the frontend: "open_url" | "retry" | "open_scaffold"
    /// | "copy" | "run_step" — extend as needed.
    pub kind: String,
    /// Free-form payload the frontend interprets per-kind (URL for open_url,
    /// text for copy, compose/dockerfile key for open_scaffold, etc.).
    pub payload: Option<String>,
}

/// Scan text for a known error pattern. First match wins so the catalog
/// order matters — put narrower patterns above broader ones.
pub fn match_error(text: &str) -> Option<BishopError> {
    for entry in CATALOG.iter() {
        if entry.pattern.is_match(text) {
            return Some(entry.to_bishop(text));
        }
    }
    None
}

struct CatalogEntry {
    code: &'static str,
    pattern: Lazy<Regex>,
    message: &'static str,
    hint: Option<&'static str>,
    docs_url: Option<&'static str>,
    action: Option<StaticAction>,
}

struct StaticAction {
    label: &'static str,
    kind: &'static str,
    payload: Option<&'static str>,
}

impl CatalogEntry {
    fn to_bishop(&self, text: &str) -> BishopError {
        BishopError {
            code: self.code.to_string(),
            message: self.message.to_string(),
            hint: self.hint.map(|s| s.to_string()),
            docs_url: self.docs_url.map(|s| s.to_string()),
            action: self.action.as_ref().map(|a| BishopAction {
                label: a.label.to_string(),
                kind: a.kind.to_string(),
                payload: a.payload.map(|s| s.to_string()),
            }),
            raw: Some(tail_lines(text, 20)),
        }
    }
}

/// Keep just the last `n` non-blank lines — the user only needs the tail
/// where the actual error printed, not the whole deploy log.
fn tail_lines(text: &str, n: usize) -> String {
    let mut kept: Vec<&str> = text.lines().rev().filter(|l| !l.trim().is_empty()).take(n).collect();
    kept.reverse();
    kept.join("\n")
}

macro_rules! entry {
    (
        code: $code:expr,
        pattern: $pat:expr,
        message: $msg:expr
        $(, hint: $hint:expr)?
        $(, docs_url: $docs:expr)?
        $(, action: { label: $lbl:expr, kind: $knd:expr $(, payload: $pld:expr)? })?
        $(,)?
    ) => {
        CatalogEntry {
            code: $code,
            pattern: Lazy::new(|| Regex::new($pat).expect("bad regex in error catalog")),
            message: $msg,
            hint: entry!(@opt $($hint)?),
            docs_url: entry!(@opt $($docs)?),
            action: entry!(@action $($lbl, $knd $(, $pld)?)?),
        }
    };
    (@opt) => { None };
    (@opt $v:expr) => { Some($v) };
    (@action) => { None };
    (@action $lbl:expr, $knd:expr) => {
        Some(StaticAction { label: $lbl, kind: $knd, payload: None })
    };
    (@action $lbl:expr, $knd:expr, $pld:expr) => {
        Some(StaticAction { label: $lbl, kind: $knd, payload: Some($pld) })
    };
}

/// The catalog. Narrower patterns first. Every entry should have a hint —
/// the whole point is "here's what to do about it".
static CATALOG: Lazy<Vec<CatalogEntry>> = Lazy::new(|| vec![
    entry! {
        code: "SSH_PERMISSION_DENIED",
        pattern: r"(?i)Permission denied \(publickey",
        message: "The server refused your SSH key.",
        hint: "Make sure your key is added to the server's authorized_keys, and that ssh-agent has it loaded (run `ssh-add -l`). If you're using an IdentityFile from ~/.ssh/config, confirm the path is correct.",
        docs_url: "https://docs.github.com/en/authentication/troubleshooting-ssh",
    },
    entry! {
        code: "SSH_HOST_UNREACHABLE",
        pattern: r"(?i)(No route to host|Network is unreachable|Could not resolve hostname|Connection timed out)",
        message: "Can't reach the server over SSH.",
        hint: "Check your internet connection and the hostname/IP in the environment config. If the server is behind a firewall, make sure port 22 (or your custom SSH port) is open.",
    },
    entry! {
        code: "DOCKER_NOT_INSTALLED",
        pattern: r"(?i)(docker: command not found|bash: docker: command not found|command not found: docker)",
        message: "Docker isn't installed on the server.",
        hint: "Run `setup-server` first — it installs Docker, Traefik, and the shared Postgres/Redis stack.",
        action: { label: "Run setup-server", kind: "run_step", payload: "setup-server" },
    },
    entry! {
        code: "PORT_ALREADY_IN_USE",
        pattern: r"(?i)(Bind for 0\.0\.0\.0:\d+ failed: port is already allocated|address already in use|bind: address already in use)",
        message: "The port this app wants is already taken on the server.",
        hint: "Another container (or the host OS) is holding that port. SSH in and run `sudo lsof -iTCP -sTCP:LISTEN` to find the culprit. In most Bishop setups only Traefik binds to 80/443 — you shouldn't need to publish app ports.",
    },
    entry! {
        code: "COMPOSE_FILE_MISSING",
        pattern: r"(?i)(docker-compose\.prod\.yml.*(?:no such file|not found|does not exist)|scp:.*docker-compose\.prod\.yml: No such file)",
        message: "docker-compose.prod.yml is missing from this project.",
        hint: "Bishop can scaffold one for you with sensible defaults — review it before deploying.",
        action: { label: "Open scaffold", kind: "open_scaffold", payload: "compose" },
    },
    entry! {
        code: "ENV_FILE_MISSING",
        pattern: r"(?i)(\.env.*(?:no such file|not found)|env file .* not found|failed to open env file)",
        message: "The .env file for this environment isn't on the server.",
        hint: "Run `setup-app` to bootstrap the app directory and upload environment variables. Or open the Environment Variables panel to set them from Bishop.",
        action: { label: "Run setup-app", kind: "run_step", payload: "setup-app" },
    },
    entry! {
        code: "GHCR_LOGIN_FAILED",
        pattern: r"(?i)(denied: requested access to the resource is denied|unauthorized: authentication required|docker login.* failed)",
        message: "GitHub Container Registry rejected the login.",
        hint: "Your GHCR_TOKEN is missing, expired, or doesn't have `read:packages` scope. Generate a new classic token at github.com/settings/tokens with read:packages and write:packages, then paste it into the environment's secrets.",
        docs_url: "https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry",
    },
    entry! {
        code: "DISK_FULL",
        pattern: r"(?i)(no space left on device|write error: No space left)",
        message: "The server's disk is full.",
        hint: "Run `docker system prune -af --volumes` on the server to clear unused images + volumes. If it's still full, the database volume has probably grown — check `df -h` and `du -sh /var/lib/docker/volumes/*`.",
    },
    entry! {
        code: "TRAEFIK_CERT_STUCK",
        pattern: r"(?i)(acme: error.*authorization|unable to generate a certificate|too many certificates already issued)",
        message: "Traefik couldn't issue a TLS certificate from Let's Encrypt.",
        hint: "Almost always a DNS problem: make sure the domain's A record points at the server's public IP and has propagated. Let's Encrypt also rate-limits to 50 certs/domain/week — if you're hitting that, wait or use staging ACME for testing.",
        docs_url: "https://letsencrypt.org/docs/rate-limits/",
    },
    entry! {
        code: "BASH_UNBOUND_VARIABLE",
        pattern: r"bash: line \d+: \$\w+: unbound variable",
        message: "The deploy script hit an unset variable.",
        hint: "Usually means a required env var isn't set on the server, or a template left a placeholder. Check the line number in the output above — it'll tell you which variable. If it's `$6` or similar, that's a missing positional arg, usually from the setup-server invocation.",
    },
]);
