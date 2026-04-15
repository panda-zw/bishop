//! Centralized on-disk paths used by Bishop, with a one-time migration
//! from the old `overlord/` directories (pre-rename) so existing users
//! don't lose their settings, scrollback, or history.
//!
//! Call `migrate_legacy_dirs()` once at app startup before anything reads
//! from the config dir.

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

/// `~/Library/Application Support/bishop/` on macOS, equivalent on other OSes.
pub fn config_dir() -> Result<PathBuf, anyhow::Error> {
    let base = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("no config dir"))?
        .join("bishop");
    fs::create_dir_all(&base)?;
    Ok(base)
}

/// `~/.ssh/bishop/` — SSH ControlMaster socket directory. Short on purpose so
/// the socket path stays under macOS's 104-byte Unix-socket limit.
pub fn ssh_control_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let d = dirs::home_dir()
            .map(|h| h.join(".ssh").join("bishop"))
            .unwrap_or_else(|| std::env::temp_dir().join("bishop"));
        let _ = fs::create_dir_all(&d);
        d
    })
    .clone()
}

/// Move legacy `overlord/` directories to their new `bishop/` locations on
/// first launch after the rename. Idempotent: runs in the first call only,
/// subsequent calls are no-ops because the old dirs don't exist.
///
/// Migrates:
///   ~/Library/Application Support/overlord/ → .../bishop/
///   ~/.ssh/overlord/                         → ~/.ssh/bishop/
pub fn migrate_legacy_dirs() {
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        if let Some(base) = dirs::config_dir() {
            let old = base.join("overlord");
            let new = base.join("bishop");
            if old.exists() && !new.exists() {
                if let Err(e) = fs::rename(&old, &new) {
                    tracing::warn!("failed to migrate config dir {} → {}: {}", old.display(), new.display(), e);
                }
            }
        }
        if let Some(home) = dirs::home_dir() {
            let old = home.join(".ssh").join("overlord");
            let new = home.join(".ssh").join("bishop");
            if old.exists() && !new.exists() {
                if let Err(e) = fs::rename(&old, &new) {
                    tracing::warn!("failed to migrate ssh control dir {} → {}: {}", old.display(), new.display(), e);
                }
            }
        }
    });
}
