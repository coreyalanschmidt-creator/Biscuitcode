//! `biscuitcode-db` — SQLite-backed conversation persistence.
//!
//! Phase 5 deliverable. Design contract:
//!  - `docs/design/AGENT-LOOP.md` § Conversation persistence
//!  - `docs/CONVERSATION-EXPORT-SCHEMA.md`
//!
//! Public surface:
//!  - [`Database`] — opens / migrates the SQLite file
//!  - [`Workspace`], [`Conversation`], [`StoredMessage`] — row types
//!  - [`Snapshot`], [`SnapshotFile`] — Phase 6b rewind manifests
//!  - [`DbError`] — internal errors (catalogue mapping at the IPC layer)
//!
//! Migrations live in `src/migrations/` as `.sql` files included via
//! `include_str!`; the migrator runs them in `PRAGMA user_version` order.

#![warn(missing_docs)]

pub mod migrations;
pub mod types;

pub use types::{
    Conversation, ConversationId, DbError, Snapshot, SnapshotFile, SnapshotId,
    StoredMessage, MessageId, Workspace, WorkspaceId,
};

use std::path::Path;

use rusqlite::Connection;

/// Convenience handle to the open SQLite connection. Single-threaded for
/// simplicity in v1; if write contention becomes an issue in v1.1 we
/// switch to a connection pool.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the SQLite file at the given path. Runs all
    /// pending migrations before returning.
    ///
    /// Sets WAL mode + busy_timeout 5s + foreign_keys on. These three
    /// together cover 95% of "sqlite is mysteriously locked" issues.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5_000)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let mut db = Self { conn };
        migrations::run(&mut db.conn)?;
        Ok(db)
    }

    /// For tests: open an in-memory database.
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let mut db = Self { conn };
        migrations::run(&mut db.conn)?;
        Ok(db)
    }

    /// Borrow the inner connection. Phase 5 coder uses this from
    /// repository methods; v1.1 may switch to typed query objects.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Borrow the inner connection mutably (transactions, migrations).
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_runs_migrations() {
        let db = Database::open_in_memory().expect("in-memory open should succeed");
        // The migrator should have set user_version to the latest.
        let v: u32 = db
            .conn()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("user_version readable");
        assert!(v >= 1, "expected at least migration 1 to have run");
    }

    #[test]
    fn schema_has_workspaces_conversations_messages_tables() {
        let db = Database::open_in_memory().unwrap();
        let names: Vec<String> = db
            .conn()
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        for required in ["conversations", "messages", "workspaces", "snapshots", "snapshot_files"] {
            assert!(
                names.iter().any(|n| n == required),
                "missing table {required}; have {names:?}",
            );
        }
    }
}
