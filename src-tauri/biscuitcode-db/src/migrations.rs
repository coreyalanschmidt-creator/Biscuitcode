//! Hand-rolled migration runner using `PRAGMA user_version`.
//!
//! Migrations are `&str` constants (embedded via `include_str!` for
//! readability) registered in the `MIGRATIONS` slice. Each migration
//! runs exactly once, in order, when its target version is greater
//! than the database's current `user_version`.
//!
//! Adding a new migration: append a new tuple to `MIGRATIONS`. NEVER
//! reorder or modify existing entries — that would silently rewrite
//! history on existing installations.

use rusqlite::Connection;

use crate::types::DbError;

/// (target_user_version, sql_script). MUST be sorted by target_version
/// ascending. Each script may contain multiple statements separated by `;`.
static MIGRATIONS: &[(u32, &str)] = &[
    (1, include_str!("migrations/0001_initial.sql")),
    // (2, include_str!("migrations/0002_…sql")),  // append; never edit (1)
];

/// The highest schema version this binary knows how to handle. Used to
/// detect "user has a newer .db than this app" and refuse rather than
/// silently losing data.
pub const MAX_SCHEMA_VERSION: u32 = 1;

/// Apply every migration with target version > current `user_version`.
pub fn run(conn: &mut Connection) -> Result<(), DbError> {
    let current: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current > MAX_SCHEMA_VERSION {
        return Err(DbError::SchemaTooNew {
            found: current,
            max: MAX_SCHEMA_VERSION,
        });
    }

    for (target, sql) in MIGRATIONS {
        if *target <= current {
            continue;
        }
        let tx = conn.transaction().map_err(DbError::from)?;
        tx.execute_batch(sql).map_err(|e| DbError::Migration {
            version: *target,
            reason: format!("{e}"),
        })?;
        tx.pragma_update(None, "user_version", target)?;
        tx.commit().map_err(DbError::from)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_db_runs_all_migrations() {
        let mut conn = Connection::open_in_memory().unwrap();
        run(&mut conn).unwrap();
        let v: u32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, MAX_SCHEMA_VERSION);
    }

    #[test]
    fn double_run_is_a_noop() {
        let mut conn = Connection::open_in_memory().unwrap();
        run(&mut conn).unwrap();
        run(&mut conn).unwrap(); // should NOT replay migrations
        let v: u32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, MAX_SCHEMA_VERSION);
    }

    #[test]
    fn schema_too_new_errors() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", MAX_SCHEMA_VERSION + 5)
            .unwrap();
        let err = run(&mut conn).unwrap_err();
        match err {
            DbError::SchemaTooNew { found, max } => {
                assert_eq!(found, MAX_SCHEMA_VERSION + 5);
                assert_eq!(max, MAX_SCHEMA_VERSION);
            }
            other => panic!("expected SchemaTooNew, got {other:?}"),
        }
    }
}
