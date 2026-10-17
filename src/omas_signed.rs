//! Signed OMAS admission. The signature binds the operation, tenant, target,
 //! resource quota, payload hash and nonce before durable replay admission.
use crate::omas_policy;
use openssl::pkey::{Id, PKey};
use openssl::sign::Verifier;
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const WINDOW_SECONDS: i64 = 300;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedOperation {
    pub tenant: String,
    pub namespace: String,
    pub target: String,
    pub opcode: u8,
    pub fuel: u64,
    pub issued_at: i64,
    pub nonce: String,
    pub payload: Vec<u8>,
    pub signature: Vec<u8>,
}
#[derive(Debug, thiserror::Error)]
pub enum SignedOperationError {
    #[error("invalid operation or security policy")]
    Invalid,
    #[error("signature verification failed")]
    Signature,
    #[error("replay detected")]
    Replay,
    #[error("replay database unavailable")]
    Store,
}
fn field(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(&(value.len() as u64).to_be_bytes());
    out.extend_from_slice(value);
}
impl SignedOperation {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"OCAF-OMAS-SIGNED-OPERATION-v1".to_vec();
        field(&mut out, self.tenant.as_bytes());
        field(&mut out, self.namespace.as_bytes());
        field(&mut out, self.target.as_bytes());
        field(&mut out, &[self.opcode]);
        field(&mut out, &self.fuel.to_be_bytes());
        field(&mut out, &self.issued_at.to_be_bytes());
        field(&mut out, self.nonce.as_bytes());
        field(&mut out, &Sha256::digest(&self.payload));
        out
    }
}
/// Verify first, then atomically consume a nonce in a persistent SQLite store.
/// On database failure, execution must not proceed.
pub fn verify_and_consume(
    operation: &SignedOperation,
    public_key: &[u8; 32],
    database: &Path,
    now: i64,
) -> Result<(), SignedOperationError> {
    if !(1..=5).contains(&operation.opcode)
        || !omas_policy::admit(
            &operation.tenant, &operation.namespace, &operation.target,
            operation.payload.len().saturating_add(4096), operation.payload.len(), operation.fuel,
        )
        || operation.nonce.len() < 16 || operation.nonce.len() > 128
        || !operation.nonce.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || operation.signature.len() != 64
        || operation.issued_at > now.saturating_add(WINDOW_SECONDS)
        || operation.issued_at < now.saturating_sub(WINDOW_SECONDS)
    {
        return Err(SignedOperationError::Invalid);
    }
    let key = PKey::public_key_from_raw_bytes(public_key, Id::ED25519)
        .map_err(|_| SignedOperationError::Signature)?;
    let mut verifier = Verifier::new_without_digest(&key)
        .map_err(|_| SignedOperationError::Signature)?;
    if !verifier.verify_oneshot(&operation.signature, &operation.canonical_bytes())
        .map_err(|_| SignedOperationError::Signature)? {
        return Err(SignedOperationError::Signature);
    }
    // SQLITE_OPEN_NOFOLLOW makes the final-component symlink check atomic
    // with SQLite's open. Protect all ancestor directories and journal sidecars
    // with operator-controlled filesystem permissions.
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
        | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW;
    let mut conn = Connection::open_with_flags(database, flags)
        .map_err(|_| SignedOperationError::Store)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| SignedOperationError::Store)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| SignedOperationError::Store)?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS omas_nonce (
        tenant TEXT NOT NULL, nonce TEXT NOT NULL, expires_at INTEGER NOT NULL,
        PRIMARY KEY (tenant, nonce)
    );").map_err(|_| SignedOperationError::Store)?;
    // Retain nonces through the entire signature validity interval.
    tx.execute("DELETE FROM omas_nonce WHERE expires_at < ?1", params![now])
        .map_err(|_| SignedOperationError::Store)?;
    let expires_at = operation.issued_at.saturating_add(WINDOW_SECONDS);
    let inserted = tx.execute(
        "INSERT OR IGNORE INTO omas_nonce (tenant, nonce, expires_at) VALUES (?1, ?2, ?3)",
        params![operation.tenant, operation.nonce, expires_at],
    ).map_err(|_| SignedOperationError::Store)?;
    if inserted != 1 { return Err(SignedOperationError::Replay); }
    tx.commit().map_err(|_| SignedOperationError::Store)?;
    Ok(())
}
pub fn unix_now() -> Result<i64, SignedOperationError> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|_| SignedOperationError::Invalid)?.as_secs();
    i64::try_from(seconds).map_err(|_| SignedOperationError::Invalid)
}
#[cfg(test)]
mod tests {
    use super::*;
    use openssl::sign::Signer;
    #[test]
    fn concurrent_replay_allows_exactly_one_admission() {
        let key = PKey::generate_ed25519().unwrap();
        let public: [u8; 32] = key.raw_public_key().unwrap().try_into().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonce.sqlite");
        Connection::open(&path).unwrap().close().unwrap();
        let now = 1_800_000_000;
        let mut op = SignedOperation {
            tenant: "tenant_a".into(),
            namespace: "tenant_a/tools".into(),
            target: "tenant_a/tools/run".into(),
            opcode: 1,
            fuel: 100,
            issued_at: now,
            nonce: "concurrent_nonce_0001".into(),
            payload: b"hello".to_vec(),
            signature: Vec::new(),
        };
        op.signature = Signer::new_without_digest(&key)
            .unwrap()
            .sign_oneshot_to_vec(&op.canonical_bytes())
            .unwrap();
        let mut workers = Vec::new();
        for _ in 0..8 {
            let op = op.clone();
            let path = path.clone();
            workers.push(std::thread::spawn(move || {
                verify_and_consume(&op, &public, &path, now)
            }));
        }
        let outcomes: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
        assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
        assert_eq!(
            outcomes.iter().filter(|outcome| matches!(outcome, Err(SignedOperationError::Replay))).count(),
            7
        );
    }

    #[test]
    fn signature_binds_operation_and_replay_persists() {
        let key = PKey::generate_ed25519().unwrap();
        let public: [u8; 32] = key.raw_public_key().unwrap().try_into().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("replay.sqlite");
        Connection::open(&path).unwrap().close().unwrap();
        let now = 1_800_000_000;
        let mut op = SignedOperation {
            tenant: "tenant_a".into(), namespace: "tenant_a/tools".into(),
            target: "tenant_a/tools/run".into(), opcode: 1, fuel: 100,
            issued_at: now, nonce: "unique_nonce_0001".into(),
            payload: b"hello".to_vec(), signature: Vec::new(),
        };
        op.signature = Signer::new_without_digest(&key).unwrap()
            .sign_oneshot_to_vec(&op.canonical_bytes()).unwrap();
        let unrelated_key = PKey::generate_ed25519().unwrap();
        let unrelated_public: [u8; 32] = unrelated_key
            .raw_public_key().unwrap().try_into().unwrap();
        assert!(matches!(
            verify_and_consume(&op, &unrelated_public, &path, now),
            Err(SignedOperationError::Signature)
        ));
        let mut tampered = op.clone();
        tampered.opcode = 2;
        assert!(matches!(verify_and_consume(&tampered, &public, &path, now), Err(SignedOperationError::Signature)));
        assert!(verify_and_consume(&op, &public, &path, now).is_ok());
        assert!(matches!(verify_and_consume(&op, &public, &path, now), Err(SignedOperationError::Replay)));
        let mut other = op.clone();
        other.nonce = "unique_nonce_0002".into();
        other.signature = Signer::new_without_digest(&key).unwrap()
            .sign_oneshot_to_vec(&other.canonical_bytes()).unwrap();
        assert!(matches!(verify_and_consume(&other, &public, &dir.path().join("missing/db"), now), Err(SignedOperationError::Store)));
        let link = dir.path().join("linked.sqlite");
        #[cfg(unix)] {
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert!(matches!(verify_and_consume(&other, &public, &link, now), Err(SignedOperationError::Store)));
        }
        let mut changed = other.clone();
        changed.fuel = 101;
        assert!(matches!(verify_and_consume(&changed, &public, &path, now), Err(SignedOperationError::Signature)));
        let mut changed = other.clone();
        changed.target = "tenant_a/tools/other".into();
        assert!(matches!(verify_and_consume(&changed, &public, &path, now), Err(SignedOperationError::Signature)));
        let mut changed = other.clone();
        changed.namespace = "tenant_a".into();
        assert!(matches!(verify_and_consume(&changed, &public, &path, now), Err(SignedOperationError::Signature)));
        let mut changed = other.clone();
        changed.tenant = "tenant_b".into();
        assert!(matches!(verify_and_consume(&changed, &public, &path, now), Err(SignedOperationError::Invalid)));
        let mut changed = other.clone();
        changed.payload.push(42);
        assert!(matches!(verify_and_consume(&changed, &public, &path, now), Err(SignedOperationError::Signature)));
        assert!(matches!(verify_and_consume(&other, &public, &path, now + 301), Err(SignedOperationError::Invalid)));
        assert!(verify_and_consume(&other, &public, &path, now).is_ok());
        // A second SQLite connection sees the committed replay record.
        assert!(matches!(verify_and_consume(&other, &public, &path, now), Err(SignedOperationError::Replay)));

    }
}
