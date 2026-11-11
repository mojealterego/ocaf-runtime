//! SQLite-backed TRS-SEAL replay admission. The MAC and fencing epoch are
//! checked before the replay marker is durably committed. Never dispatch an
//! effect until this function returns Ok. A single SQLite file is NOT a
//! distributed fencing authority; the caller must supply its trusted epoch.
use crate::trs_seal::{Envelope, SealError, SealKeys, SealVerifier};
use rusqlite::{params, Connection, ErrorCode, OptionalExtension, TransactionBehavior};

#[derive(Debug)]
pub enum DurableSealError {
    Seal(SealError),
    Storage(rusqlite::Error),
    Replay,
    SequenceExhausted,
}
impl From<SealError> for DurableSealError {
    fn from(value: SealError) -> Self { Self::Seal(value) }
}
impl From<rusqlite::Error> for DurableSealError {
    fn from(value: rusqlite::Error) -> Self { Self::Storage(value) }
}

/// PRAGMA synchronous=FULL is required for crash durability on supported
/// SQLite filesystems. The caller must configure filesystem durability.
pub fn initialize(conn: &Connection) -> Result<(), DurableSealError> {
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA synchronous=FULL;
         CREATE TABLE IF NOT EXISTS ocaf_trs_fencing (\n           tenant TEXT PRIMARY KEY NOT NULL,\n           epoch TEXT NOT NULL\n         ) WITHOUT ROWID;\n         CREATE TABLE IF NOT EXISTS ocaf_trs_replay (
           tenant TEXT NOT NULL,
           sender TEXT NOT NULL,
           key_epoch TEXT NOT NULL,
           fencing_epoch TEXT NOT NULL,
           sequence TEXT NOT NULL,
           PRIMARY KEY(tenant,sender,key_epoch,fencing_epoch,sequence)
         ) WITHOUT ROWID;"
    )?;
    Ok(())
}

/// Low-level verification only: does NOT bind an authenticated session identity.\n/// Prefer verify_and_record_authorized for network-facing admission.\n/// Verify the authenticated envelope and atomically claim its replay identity.
/// This function does not decrypt the payload and is not an authorization check.
pub fn verify_and_record(
    conn: &mut Connection,
    keys: &SealKeys,
    trusted_fencing_epoch: u64,
    envelope: &Envelope,
) -> Result<(), DurableSealError> {
    // A fresh verifier validates MAC and epoch without updating any persistent
    // replay state. SQLite's unique index is the authoritative replay barrier.
    let mut authentication = SealVerifier::new(trusted_fencing_epoch, 1)?;
    authentication.verify(keys, envelope)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let result = tx.execute(
        "INSERT INTO ocaf_trs_replay(tenant,sender,key_epoch,fencing_epoch,sequence)
         VALUES(?1,?2,?3,?4,?5)",
        params![
            envelope.tenant, envelope.sender,
            envelope.key_epoch.to_string(), envelope.fencing_epoch.to_string(),
            envelope.sequence.to_string()
        ],
    );
    match result {
        Ok(1) => { tx.commit()?; Ok(()) }
        Ok(_) => Err(DurableSealError::Replay),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == ErrorCode::ConstraintViolation => Err(DurableSealError::Replay),
        Err(e) => Err(DurableSealError::Storage(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn keys() -> SealKeys { SealKeys::new(2, &[0x55; 32]).unwrap() }

    #[test]
    fn durable_replay_survives_new_verifier_and_connection() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut first = Connection::open(file.path()).unwrap();
        initialize(&first).unwrap();
        let keys = keys();
        let message = keys.sign("tenant-a", "sender-1", 9, 12, b"data").unwrap();
        verify_and_record(&mut first, &keys, 9, &message).unwrap();
        drop(first);
        let mut second = Connection::open(file.path()).unwrap();
        assert!(matches!(verify_and_record(&mut second, &keys, 9, &message), Err(DurableSealError::Replay)));
    }

    #[test]
    fn tampering_does_not_consume_sequence() {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        let keys = keys();
        let message = keys.sign("tenant-a", "sender-1", 9, 12, b"data").unwrap();
        let mut bad = message.clone();
        bad.payload.push(0xff);
        assert!(matches!(verify_and_record(&mut conn, &keys, 9, &bad), Err(DurableSealError::Seal(SealError::InvalidTag))));
        verify_and_record(&mut conn, &keys, 9, &message).unwrap();
    }

    #[test]
    fn fencing_mismatch_is_rejected_without_db_write() {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        let keys = keys();
        let message = keys.sign("tenant-a", "sender-1", 8, 12, b"data").unwrap();
        assert!(matches!(verify_and_record(&mut conn, &keys, 9, &message), Err(DurableSealError::Seal(SealError::UnauthorizedFencingEpoch))));
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM ocaf_trs_replay", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn large_u64_values_do_not_collide_in_sqlite() {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        let keys = keys();
        let a = keys.sign("tenant-a", "sender-1", u64::MAX, u64::MAX, b"a").unwrap();
        let b = keys.sign("tenant-a", "sender-1", u64::MAX, u64::MAX - 1, b"b").unwrap();
        verify_and_record(&mut conn, &keys, u64::MAX, &a).unwrap();
        verify_and_record(&mut conn, &keys, u64::MAX, &b).unwrap();
    }
}


/// Persist an externally authorized fencing epoch. Never derive the new epoch
/// from untrusted network messages. Repeated or rollback epochs fail closed.
pub fn advance_fencing_epoch(
    conn: &mut Connection,
    tenant: &str,
    new_epoch: u64,
) -> Result<(), DurableSealError> {
    if tenant.is_empty() || tenant.len() > 256 || tenant.chars().any(char::is_control) {
        return Err(DurableSealError::Seal(SealError::InvalidIdentity));
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let old: Option<String> = tx.query_row(
        "SELECT epoch FROM ocaf_trs_fencing WHERE tenant=?1",
        params![tenant],
        |row| row.get(0),
    ).optional()?;
    if let Some(old) = old {
        let old = old.parse::<u64>().map_err(|_| DurableSealError::Seal(SealError::EpochRollback))?;
        if new_epoch <= old {
            return Err(DurableSealError::Seal(SealError::EpochRollback));
        }
        tx.execute(
            "UPDATE ocaf_trs_fencing SET epoch=?1 WHERE tenant=?2",
            params![new_epoch.to_string(), tenant],
        )?;
    } else {
        tx.execute(
            "INSERT INTO ocaf_trs_fencing(tenant,epoch) VALUES(?1,?2)",
            params![tenant, new_epoch.to_string()],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Admission with durable epoch pinning: a caller-supplied old epoch cannot
/// override the epoch already stored for this tenant. Authenticate and commit
/// replay identity under one IMMEDIATE transaction.
pub fn verify_and_record_pinned(
    conn: &mut Connection,
    keys: &SealKeys,
    envelope: &Envelope,
) -> Result<(), DurableSealError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let stored: Option<String> = tx.query_row(
        "SELECT epoch FROM ocaf_trs_fencing WHERE tenant=?1",
        params![envelope.tenant],
        |row| row.get(0),
    ).optional()?;
    let stored = stored.ok_or(DurableSealError::Seal(SealError::UnauthorizedFencingEpoch))?;
    let epoch = stored.parse::<u64>().map_err(|_| DurableSealError::Seal(SealError::EpochRollback))?;
    let mut authentication = SealVerifier::new(epoch, 1)?;
    authentication.verify(keys, envelope)?;
    let result = tx.execute(
        "INSERT INTO ocaf_trs_replay(tenant,sender,key_epoch,fencing_epoch,sequence)
         VALUES(?1,?2,?3,?4,?5)",
        params![envelope.tenant, envelope.sender, envelope.key_epoch.to_string(),
                envelope.fencing_epoch.to_string(), envelope.sequence.to_string()],
    );
    match result {
        Ok(1) => { tx.commit()?; Ok(()) }
        Ok(_) => Err(DurableSealError::Replay),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == ErrorCode::ConstraintViolation => Err(DurableSealError::Replay),
        Err(e) => Err(DurableSealError::Storage(e)),
    }
}

#[cfg(test)]
mod fencing_tests {
    use super::*;

    #[test]
    fn persisted_fencing_epoch_blocks_old_leader_after_restart() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let keys = SealKeys::new(1, &[7u8; 32]).unwrap();
        let old = keys.sign("tenant-a", "leader", 4, 1, b"old").unwrap();
        let current = keys.sign("tenant-a", "leader", 5, 2, b"new").unwrap();
        {
            let mut conn = Connection::open(file.path()).unwrap();
            initialize(&conn).unwrap();
            advance_fencing_epoch(&mut conn, "tenant-a", 4).unwrap();
            verify_and_record_pinned(&mut conn, &keys, &old).unwrap();
            advance_fencing_epoch(&mut conn, "tenant-a", 5).unwrap();
        }
        let mut conn = Connection::open(file.path()).unwrap();
        assert!(matches!(
            verify_and_record_pinned(&mut conn, &keys, &old),
            Err(DurableSealError::Seal(SealError::UnauthorizedFencingEpoch))
        ));
        verify_and_record_pinned(&mut conn, &keys, &current).unwrap();
        assert!(matches!(
            advance_fencing_epoch(&mut conn, "tenant-a", 4),
            Err(DurableSealError::Seal(SealError::EpochRollback))
        ));
    }

    #[test]
    fn missing_authority_record_is_fail_closed() {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        let keys = SealKeys::new(1, &[7u8; 32]).unwrap();
        let msg = keys.sign("tenant-a", "leader", 1, 1, b"payload").unwrap();
        assert!(matches!(
            verify_and_record_pinned(&mut conn, &keys, &msg),
            Err(DurableSealError::Seal(SealError::UnauthorizedFencingEpoch))
        ));
    }
}

/// Authenticate a message against the caller's authorized tenant and sender
/// before durable replay admission. The identities must come from the
/// authenticated transport/session, never from the envelope itself.
pub fn verify_and_record_authorized(
    conn: &mut Connection,
    keys: &SealKeys,
    authorized_tenant: &str,
    authorized_sender: &str,
    envelope: &Envelope,
) -> Result<(), DurableSealError> {
    if envelope.tenant != authorized_tenant || envelope.sender != authorized_sender {
        return Err(DurableSealError::Seal(SealError::InvalidIdentity));
    }
    verify_and_record_pinned(conn, keys, envelope)
}

#[cfg(test)]
mod authorization_tests {
    use super::*;

    #[test]
    fn valid_mac_cannot_cross_authenticated_tenant_boundary() {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        advance_fencing_epoch(&mut conn, "tenant-b", 4).unwrap();
        let keys = SealKeys::new(1, &[0x33; 32]).unwrap();
        let message = keys.sign("tenant-b", "sender", 4, 1, b"payload").unwrap();
        assert!(matches!(
            verify_and_record_authorized(&mut conn, &keys, "tenant-a", "sender", &message),
            Err(DurableSealError::Seal(SealError::InvalidIdentity))
        ));
        verify_and_record_authorized(&mut conn, &keys, "tenant-b", "sender", &message).unwrap();
    }

    #[test]
    fn valid_mac_cannot_impersonate_another_authenticated_sender() {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        advance_fencing_epoch(&mut conn, "tenant-a", 4).unwrap();
        let keys = SealKeys::new(1, &[0x33; 32]).unwrap();
        let message = keys.sign("tenant-a", "sender-b", 4, 1, b"payload").unwrap();
        assert!(matches!(
            verify_and_record_authorized(&mut conn, &keys, "tenant-a", "sender-a", &message),
            Err(DurableSealError::Seal(SealError::InvalidIdentity))
        ));
        verify_and_record_authorized(&mut conn, &keys, "tenant-a", "sender-b", &message).unwrap();
    }
}
