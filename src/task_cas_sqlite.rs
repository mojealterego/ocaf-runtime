//! Durable single-node CAS using SQLite. State update and audit receipt are
//! committed atomically. This is NOT PostgreSQL HA or a quorum fencing service.
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use crate::task_cas::{CasError, TaskAggregate, TaskState, TransitionReceipt};

#[derive(Debug)]
pub enum DurableCasError {
    Database(rusqlite::Error),
    Conflict,
    Domain(CasError),
    CorruptState,
}
impl From<rusqlite::Error> for DurableCasError {
    fn from(e: rusqlite::Error) -> Self { Self::Database(e) }
}
impl From<CasError> for DurableCasError {
    fn from(e: CasError) -> Self { Self::Domain(e) }
}

fn encode(state: TaskState) -> &'static str {
    match state {
        TaskState::Created => "Created",
        TaskState::Queued => "Queued",
        TaskState::Leased => "Leased",
        TaskState::Running => "Running",
        TaskState::Verifying => "Verifying",
        TaskState::Succeeded => "Succeeded",
        TaskState::Failed => "Failed",
        TaskState::Cancelled => "Cancelled",
    }
}
fn decode(s: &str) -> Result<TaskState, DurableCasError> {
    Ok(match s {
        "Created" => TaskState::Created,
        "Queued" => TaskState::Queued,
        "Leased" => TaskState::Leased,
        "Running" => TaskState::Running,
        "Verifying" => TaskState::Verifying,
        "Succeeded" => TaskState::Succeeded,
        "Failed" => TaskState::Failed,
        "Cancelled" => TaskState::Cancelled,
        _ => return Err(DurableCasError::CorruptState),
    })
}
fn to_sql_integer(value: u64) -> Result<i64, DurableCasError> {
    i64::try_from(value).map_err(|_| DurableCasError::CorruptState)
}
fn from_sql_integer(value: i64) -> Result<u64, DurableCasError> {
    u64::try_from(value).map_err(|_| DurableCasError::CorruptState)
}

pub fn initialize(conn: &Connection) -> Result<(), DurableCasError> {
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         CREATE TABLE IF NOT EXISTS ocaf_task_cas (
           task_id TEXT PRIMARY KEY NOT NULL,
           state TEXT NOT NULL,
           revision INTEGER NOT NULL CHECK(revision >= 0),
           fencing_epoch INTEGER NOT NULL CHECK(fencing_epoch >= 0)
         );
         CREATE TABLE IF NOT EXISTS ocaf_task_receipts (
           task_id TEXT NOT NULL,
           revision INTEGER NOT NULL,
           previous_state TEXT NOT NULL,
           next_state TEXT NOT NULL,
           fencing_epoch INTEGER NOT NULL,
           PRIMARY KEY(task_id, revision),
           FOREIGN KEY(task_id) REFERENCES ocaf_task_cas(task_id)
         );"
    )?;
    Ok(())
}

/// Provision a new aggregate only after external tenant/actor authorization.
pub fn insert_initial(conn: &Connection, task: &TaskAggregate) -> Result<(), DurableCasError> {
    conn.execute(
        "INSERT INTO ocaf_task_cas(task_id,state,revision,fencing_epoch) VALUES(?1,?2,?3,?4)",
        params![task.task_id, encode(task.state), to_sql_integer(task.revision)?,
                to_sql_integer(task.fencing_epoch)?],
    )?;
    Ok(())
}

/// IMMEDIATE transaction serializes writers on one SQLite database. Receipt and
/// state update share a commit boundary. Never use this as a distributed leader oracle.
pub fn transition(
    conn: &mut Connection,
    task_id: &str,
    expected_revision: u64,
    authoritative_epoch: u64,
    next: TaskState,
) -> Result<TransitionReceipt, DurableCasError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let row: Option<(String, i64, i64)> = tx.query_row(
        "SELECT state,revision,fencing_epoch FROM ocaf_task_cas WHERE task_id=?1",
        params![task_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    ).optional()?;
    let (state, revision, epoch) = row.ok_or(DurableCasError::Conflict)?;
    let mut task = TaskAggregate {
        task_id: task_id.to_owned(),
        state: decode(&state)?,
        revision: from_sql_integer(revision)?,
        fencing_epoch: from_sql_integer(epoch)?,
    };
    let receipt = task.transition_cas(expected_revision, authoritative_epoch, next)?;
    let affected = tx.execute(
        "UPDATE ocaf_task_cas SET state=?1,revision=?2 WHERE task_id=?3 AND revision=?4 AND fencing_epoch=?5",
        params![encode(next), to_sql_integer(receipt.new_revision)?, task_id,
                to_sql_integer(expected_revision)?, to_sql_integer(authoritative_epoch)?],
    )?;
    if affected != 1 { return Err(DurableCasError::Conflict); }
    tx.execute(
        "INSERT INTO ocaf_task_receipts(task_id,revision,previous_state,next_state,fencing_epoch) VALUES(?1,?2,?3,?4,?5)",
        params![task_id, to_sql_integer(receipt.new_revision)?, encode(receipt.previous),
                encode(receipt.next), to_sql_integer(authoritative_epoch)?],
    )?;
    tx.commit()?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        insert_initial(&conn, &TaskAggregate {
            task_id: "tenant-a/task-1".into(),
            state: TaskState::Created,
            revision: 0,
            fencing_epoch: 7,
        }).unwrap();
        conn
    }

    #[test]
    fn commit_updates_state_and_receipt_together() {
        let mut conn = setup();
        let receipt = transition(&mut conn, "tenant-a/task-1", 0, 7, TaskState::Queued).unwrap();
        assert_eq!(receipt.previous, TaskState::Created);
        let state: (String, i64) = conn.query_row(
            "SELECT state,revision FROM ocaf_task_cas WHERE task_id='tenant-a/task-1'",
            [], |r| Ok((r.get(0)?, r.get(1)?))
        ).unwrap();
        assert_eq!(state, ("Queued".into(), 1));
        let count: i64 = conn.query_row("SELECT count(*) FROM ocaf_task_receipts", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn stale_revision_does_not_write_receipt() {
        let mut conn = setup();
        assert!(matches!(transition(&mut conn, "tenant-a/task-1", 1, 7, TaskState::Queued),
                         Err(DurableCasError::Domain(CasError::StaleRevision))));
        let count: i64 = conn.query_row("SELECT count(*) FROM ocaf_task_receipts", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn stale_epoch_does_not_write_receipt() {
        let mut conn = setup();
        assert!(matches!(transition(&mut conn, "tenant-a/task-1", 0, 8, TaskState::Queued),
                         Err(DurableCasError::Domain(CasError::StaleFencingEpoch))));
        let count: i64 = conn.query_row("SELECT count(*) FROM ocaf_task_receipts", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn duplicate_transition_is_rejected() {
        let mut conn = setup();
        transition(&mut conn, "tenant-a/task-1", 0, 7, TaskState::Queued).unwrap();
        assert!(transition(&mut conn, "tenant-a/task-1", 0, 7, TaskState::Queued).is_err());
        let count: i64 = conn.query_row("SELECT count(*) FROM ocaf_task_receipts", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }
}
