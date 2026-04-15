//! SQLite-backed deploy history. Single connection guarded by a Mutex —
//! write throughput is trivial (one row per deploy) so contention is a non-issue.
//!
//! Schema lives in [`migrations`]; this module owns the runtime connection +
//! query helpers.

mod migrations;

use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

fn db_path() -> Result<PathBuf> {
    Ok(crate::paths::config_dir()?.join("history.db"))
}

impl Db {
    pub fn open() -> Result<Self> {
        let conn = Connection::open(db_path()?)?;
        migrations::run(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub fn insert_start(
        &self,
        project_path: &str,
        env: &str,
        started_at: DateTime<Utc>,
        actor_id: &str,
    ) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO deploys (project_path, env, started_at, status, actor_id) \
             VALUES (?1, ?2, ?3, 'running', ?4)",
            params![project_path, env, started_at.to_rfc3339(), actor_id],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_finish(
        &self,
        id: i64,
        finished_at: DateTime<Utc>,
        status: &str,
        exit_code: Option<i32>,
        log: &str,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE deploys SET finished_at = ?1, status = ?2, exit_code = ?3, log = ?4 WHERE id = ?5",
            params![finished_at.to_rfc3339(), status, exit_code, log, id],
        )?;
        Ok(())
    }

    pub fn list(&self, project_path: &str, env: &str, limit: usize) -> Result<Vec<DeployRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, project_path, env, started_at, finished_at, status, exit_code
             FROM deploys WHERE project_path = ?1 AND env = ?2
             ORDER BY started_at DESC LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(params![project_path, env, limit as i64], |r| {
                Ok(DeployRow {
                    id: r.get(0)?,
                    project_path: r.get(1)?,
                    env: r.get(2)?,
                    started_at: r.get(3)?,
                    finished_at: r.get(4)?,
                    status: r.get(5)?,
                    exit_code: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_log(&self, id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT log FROM deploys WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(r) = rows.next()? { Ok(Some(r.get(0)?)) } else { Ok(None) }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct DeployRow {
    pub id: i64,
    pub project_path: String,
    pub env: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    pub exit_code: Option<i32>,
}
