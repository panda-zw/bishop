//! Append-only JSONL audit log — every deploy, rollback, secret access, and
//! env-var write gets one line. No UI in v0.1; this exists to make the
//! eventual Tier 4 audit-log surface a content problem rather than an
//! instrumentation problem. Retrofitting call sites is painful; writing a
//! line per event now is free.
//!
//! Location: <config_dir>/audit.jsonl. Never rotated here — we'll cap/rotate
//! when the file actually grows big enough to matter.

use chrono::Utc;
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::Write;

use crate::paths;

/// Write one audit line. Failures are logged but never propagated — audit is
/// best-effort and must not break the operation it's observing.
pub fn audit(kind: &str, fields: Value) {
    if let Err(e) = append(kind, fields) {
        tracing::warn!(error = %e, "audit log write failed");
    }
}

fn append(kind: &str, fields: Value) -> anyhow::Result<()> {
    let path = paths::config_dir()?.join("audit.jsonl");
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).ok(); }

    let record = json!({
        "ts": Utc::now().to_rfc3339(),
        "actor": actor_id(),
        "kind": kind,
        "fields": fields,
    });

    let mut f = OpenOptions::new().create(true).append(true).open(&path)?;
    writeln!(f, "{}", serde_json::to_string(&record)?)?;
    Ok(())
}

/// Stable identity for this machine+user. Defaults to `$USER@hostname` so
/// the column still means something when multi-user mode arrives.
pub fn actor_id() -> String {
    let user = whoami::username();
    let host = whoami::fallible::hostname().unwrap_or_else(|_| "unknown".into());
    format!("{}@{}", user, host)
}
