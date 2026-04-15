//! SQLite schema migrations.
//!
//! Design: each migration has a numeric version, a short name, and a
//! `run(conn)` closure. Applied migrations are recorded in `schema_version`
//! so the runner can skip them next launch.
//!
//! Adding a migration: append a new `Migration` to `MIGRATIONS` with the
//! next sequential `version`. Never edit or renumber an already-shipped
//! migration — users' DBs have already recorded the version and re-running
//! a changed body will not execute. If you need to fix a prior migration,
//! ship a *new* one that fixes forward.
//!
//! Migrations run inside a transaction each. A failure rolls back that
//! migration and bubbles up; the app fails to start rather than running
//! on a half-migrated DB.

use anyhow::{Context, Result};
use rusqlite::Connection;
use tracing::info;

pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub run: fn(&Connection) -> rusqlite::Result<()>,
}

/// Append-only, ordered by version. See module doc for the rules.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_schema",
        run: initial_schema,
    },
    Migration {
        version: 2,
        name: "deploys_actor_id",
        run: deploys_actor_id,
    },
];

pub fn run(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
         );",
    )?;

    let applied: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    for m in MIGRATIONS {
        if m.version <= applied { continue; }
        info!(version = m.version, name = m.name, "applying migration");
        apply_one(conn, m).with_context(|| format!("migration {} ({}) failed", m.version, m.name))?;
    }
    Ok(())
}

fn apply_one(conn: &Connection, m: &Migration) -> Result<()> {
    conn.execute_batch("BEGIN")?;
    match (m.run)(conn) {
        Ok(()) => {
            conn.execute(
                "INSERT INTO schema_version (version, name) VALUES (?1, ?2)",
                rusqlite::params![m.version, m.name],
            )?;
            conn.execute_batch("COMMIT")?;
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(anyhow::anyhow!(e))
        }
    }
}

/// SQLite lacks `ADD COLUMN IF NOT EXISTS`. This makes column-adding
/// migrations safe against DBs that already have the column from an
/// earlier best-effort migration.
fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    type_decl: &str,
) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let exists = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .any(|name| name == column);
    drop(stmt);
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, type_decl),
            [],
        )?;
    }
    Ok(())
}

// -------- individual migrations --------

/// Initial schema. CREATE IF NOT EXISTS throughout so existing DBs from
/// before the migration runner existed are no-ops here.
fn initial_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS deploys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_path TEXT NOT NULL,
            env TEXT NOT NULL,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            status TEXT NOT NULL,
            exit_code INTEGER,
            log TEXT NOT NULL DEFAULT ''
        );
        CREATE INDEX IF NOT EXISTS deploys_project_env
            ON deploys(project_path, env, started_at DESC);
        "#,
    )
}

/// Adds actor_id for team-mode-later. See principle #5 in ROADMAP.md.
fn deploys_actor_id(conn: &Connection) -> rusqlite::Result<()> {
    add_column_if_missing(conn, "deploys", "actor_id", "TEXT")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    #[test]
    fn applies_all_on_fresh_db() {
        let c = mem_conn();
        run(&c).unwrap();
        let v: u32 = c
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, MIGRATIONS.last().unwrap().version);
    }

    #[test]
    fn idempotent_second_call() {
        let c = mem_conn();
        run(&c).unwrap();
        let first_count: u32 = c
            .query_row("SELECT COUNT(*) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        run(&c).unwrap();
        let second_count: u32 = c
            .query_row("SELECT COUNT(*) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(first_count, second_count);
    }

    #[test]
    fn migrates_from_preexisting_v1_schema() {
        // Simulates a v0.1.0 user whose DB was created before the migration
        // runner existed. The initial_schema migration must be a no-op.
        let c = mem_conn();
        c.execute_batch(
            "CREATE TABLE deploys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_path TEXT NOT NULL,
                env TEXT NOT NULL,
                started_at TEXT NOT NULL,
                finished_at TEXT,
                status TEXT NOT NULL,
                exit_code INTEGER,
                log TEXT NOT NULL DEFAULT ''
            );",
        )
        .unwrap();
        run(&c).unwrap();
        let has_actor: bool = c
            .query_row(
                "SELECT 1 FROM pragma_table_info('deploys') WHERE name='actor_id'",
                [],
                |r| r.get::<_, i32>(0),
            )
            .map(|_| true)
            .unwrap_or(false);
        assert!(has_actor, "actor_id should have been added");
    }

    #[test]
    fn migrates_from_preexisting_v2_schema() {
        // Simulates a v0.1.1 user whose DB got actor_id via the old
        // add_column_if_missing stopgap without a schema_version row.
        let c = mem_conn();
        c.execute_batch(
            "CREATE TABLE deploys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_path TEXT NOT NULL,
                env TEXT NOT NULL,
                started_at TEXT NOT NULL,
                finished_at TEXT,
                status TEXT NOT NULL,
                exit_code INTEGER,
                log TEXT NOT NULL DEFAULT '',
                actor_id TEXT
            );",
        )
        .unwrap();
        // Should succeed — deploys_actor_id migration detects the column
        // and no-ops instead of failing with "duplicate column".
        run(&c).unwrap();
    }
}
