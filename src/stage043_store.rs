//! Stage 0.43 durable tool registry. This is local operator-controlled
//! metadata storage, not an execution authority or substitute for external KMS.
//! The SQLite database must be created under a protected, trusted directory.
use crate::omas_signed::{verify_signature, SignedOperation, SignedOperationError, WINDOW_SECONDS};
use crate::omat_server::{OmatRegistry, ToolManifest};
use crate::stage043_signed_manifest::{
    register_signed_manifest, SignedManifestError, SignedToolManifest,
};
use rusqlite::{
    params, Connection, Error as SqlError, ErrorCode, OpenFlags, OptionalExtension,
    TransactionBehavior,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use thiserror::Error;

const MAX_STORED_MANIFEST_BYTES: usize = 1_048_576;
const TOOL_REVOKE_OPCODE: u8 = 5;

/// The exact canonical JSON bytes of this request are bound to a V2
/// SignedOperation. The digest names the manifest currently stored, not a new
/// manifest supplied by the caller.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolRevocationRequest {
    pub tenant_id: String,
    pub tool_id: String,
    pub manifest_sha256: String,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("invalid or inaccessible operator-controlled database path")]
    Database,
    #[error("tool identity has already been recorded, including revoked entries")]
    Duplicate,
    #[error("stored manifest integrity check failed")]
    Integrity,
    #[error("tool record not found or already revoked")]
    NotFound,
    #[error("tool revocation is not bound to the authenticated request")]
    Binding,
    #[error("tool revocation nonce has already been used")]
    Replay,
    #[error("tool manifest digest changed since revocation was signed")]
    StaleDigest,
    #[error("signed tool revocation rejected: {0}")]
    Operation(#[from] SignedOperationError),
    #[error("signed tool manifest rejected: {0}")]
    Signature(#[from] SignedManifestError),
}

#[derive(Clone, Debug)]
pub struct Stage043ToolStore {
    path: PathBuf,
}

impl Stage043ToolStore {
    /// Reject missing databases and symlink final components. Provisioning and
    /// permissions belong to the operator; this API never creates a parent dir.
    pub fn open_existing(path: &Path) -> Result<Self, StoreError> {
        if !path.is_absolute() || !path.is_file() {
            return Err(StoreError::Database);
        }
        let store = Self {
            path: path.to_owned(),
        };
        let connection = store.connection()?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS stage043_tools (
                tenant TEXT NOT NULL,
                tool_id TEXT NOT NULL,
                version TEXT NOT NULL,
                digest TEXT NOT NULL,
                manifest BLOB NOT NULL,
                revoked INTEGER NOT NULL DEFAULT 0 CHECK(revoked IN (0, 1)),
                PRIMARY KEY(tenant, tool_id)
            );
            CREATE TABLE IF NOT EXISTS stage043_revoke_nonce (
                tenant TEXT NOT NULL,
                nonce TEXT NOT NULL,
                expires_at INTEGER NOT NULL,
                PRIMARY KEY(tenant, nonce)
            );",
            )
            .map_err(|_| StoreError::Database)?;
        Ok(store)
    }

    fn connection(&self) -> Result<Connection, StoreError> {
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let connection =
            Connection::open_with_flags(&self.path, flags).map_err(|_| StoreError::Database)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| StoreError::Database)?;
        Ok(connection)
    }

    /// Validate signature, trusted tenant and lifetime before the atomic insert.
    /// A revoked identity is a permanent tombstone; no silent reactivation.
    pub fn register_signed(
        &self,
        envelope: &SignedToolManifest,
        trusted_tenant: &str,
        trusted_key: &[u8; 32],
        now: i64,
    ) -> Result<String, StoreError> {
        let mut verified = OmatRegistry::default();
        let digest =
            register_signed_manifest(&mut verified, envelope, trusted_tenant, trusted_key, now)?;
        let encoded = serde_json::to_vec(&envelope.manifest).map_err(|_| StoreError::Integrity)?;
        if encoded.len() > MAX_STORED_MANIFEST_BYTES {
            return Err(StoreError::Integrity);
        }
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Database)?;
        match transaction.execute(
            "INSERT INTO stage043_tools
             (tenant, tool_id, version, digest, manifest, revoked)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![
                envelope.manifest.tenant_id,
                envelope.manifest.tool_id,
                envelope.manifest.version,
                digest,
                encoded,
            ],
        ) {
            Ok(1) => {}
            Err(SqlError::SqliteFailure(code, _))
                if code.code == ErrorCode::ConstraintViolation =>
            {
                return Err(StoreError::Duplicate);
            }
            _ => return Err(StoreError::Database),
        }
        transaction.commit().map_err(|_| StoreError::Database)?;
        Ok(digest)
    }

    /// The digest detects accidental/tampered raw manifest bytes; an attacker
    /// with database write access could also rewrite digest, so storage
    /// isolation and server-side trust anchors remain mandatory.
    pub fn get_active(
        &self,
        tenant: &str,
        tool_id: &str,
    ) -> Result<Option<ToolManifest>, StoreError> {
        let connection = self.connection()?;
        let result: Option<(Vec<u8>, String, i64)> = connection
            .query_row(
                "SELECT manifest, digest, revoked FROM stage043_tools
                 WHERE tenant = ?1 AND tool_id = ?2",
                params![tenant, tool_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|_| StoreError::Database)?;
        let Some((bytes, digest, revoked)) = result else {
            return Ok(None);
        };
        if bytes.len() > MAX_STORED_MANIFEST_BYTES {
            return Err(StoreError::Integrity);
        }
        let expected = format!("sha256:{:x}", Sha256::digest(&bytes));
        let tool: ToolManifest =
            serde_json::from_slice(&bytes).map_err(|_| StoreError::Integrity)?;
        if expected != digest
            || tool.tenant_id != tenant
            || tool.tool_id != tool_id
            || (revoked != 0 && revoked != 1)
        {
            return Err(StoreError::Integrity);
        }
        if revoked == 1 {
            return Ok(None);
        }
        Ok(Some(tool))
    }

    /// Revoke exactly the signed manifest for this tenant and tool. The caller
    /// must derive the sender, tenant and Ed25519 key from an authenticated,
    /// operator-managed mapping; none may come from the request body. A replay
    /// nonce and the permanent tombstone commit in one SQLite transaction.
    pub fn revoke_signed(
        &self,
        request: &ToolRevocationRequest,
        operation: &SignedOperation,
        authenticated_rpc_sender: &str,
        trusted_tenant: &str,
        trusted_key: &[u8; 32],
        now: i64,
    ) -> Result<(), StoreError> {
        let expected_namespace = format!("{trusted_tenant}/tools");
        let expected_target = format!("{expected_namespace}/control/revoke");
        let payload = serde_json::to_vec(request).map_err(|_| StoreError::Binding)?;
        let digest = request.manifest_sha256.strip_prefix("sha256:");
        if request.tenant_id != trusted_tenant
            || request.tool_id.is_empty()
            || request.tool_id.len() > 128
            || !request
                .tool_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
            || !digest.is_some_and(|hex| {
                hex.len() == 64
                    && hex
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            || authenticated_rpc_sender.is_empty()
            || operation.rpc_sender.as_deref() != Some(authenticated_rpc_sender)
            || operation.tenant != trusted_tenant
            || operation.namespace != expected_namespace
            || operation.target != expected_target
            || operation.opcode != TOOL_REVOKE_OPCODE
            || operation.fuel != 1
            || operation.payload != payload
        {
            return Err(StoreError::Binding);
        }
        verify_signature(operation, trusted_key, now)?;

        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Database)?;
        transaction
            .execute(
                "DELETE FROM stage043_revoke_nonce WHERE expires_at < ?1",
                params![now],
            )
            .map_err(|_| StoreError::Database)?;
        let inserted = transaction
            .execute(
                "INSERT OR IGNORE INTO stage043_revoke_nonce (tenant, nonce, expires_at)
                 VALUES (?1, ?2, ?3)",
                params![
                    request.tenant_id,
                    operation.nonce,
                    operation.issued_at.saturating_add(WINDOW_SECONDS),
                ],
            )
            .map_err(|_| StoreError::Database)?;
        if inserted != 1 {
            return Err(StoreError::Replay);
        }

        let current: Option<(Vec<u8>, String, i64)> = transaction
            .query_row(
                "SELECT manifest, digest, revoked FROM stage043_tools
                 WHERE tenant = ?1 AND tool_id = ?2",
                params![request.tenant_id, request.tool_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|_| StoreError::Database)?;
        let Some((bytes, stored_digest, revoked)) = current else {
            transaction.commit().map_err(|_| StoreError::Database)?;
            return Err(StoreError::NotFound);
        };
        if bytes.len() > MAX_STORED_MANIFEST_BYTES || (revoked != 0 && revoked != 1) {
            transaction.commit().map_err(|_| StoreError::Database)?;
            return Err(StoreError::Integrity);
        }
        let stored_manifest: ToolManifest = match serde_json::from_slice(&bytes) {
            Ok(manifest) => manifest,
            Err(_) => {
                transaction.commit().map_err(|_| StoreError::Database)?;
                return Err(StoreError::Integrity);
            }
        };
        if stored_digest != format!("sha256:{:x}", Sha256::digest(&bytes))
            || stored_manifest.tenant_id != request.tenant_id
            || stored_manifest.tool_id != request.tool_id
        {
            transaction.commit().map_err(|_| StoreError::Database)?;
            return Err(StoreError::Integrity);
        }
        if revoked == 1 {
            transaction.commit().map_err(|_| StoreError::Database)?;
            return Err(StoreError::NotFound);
        }
        if stored_digest != request.manifest_sha256 {
            transaction.commit().map_err(|_| StoreError::Database)?;
            return Err(StoreError::StaleDigest);
        }
        let changed = transaction
            .execute(
                "UPDATE stage043_tools SET revoked = 1
                 WHERE tenant = ?1 AND tool_id = ?2 AND digest = ?3 AND revoked = 0",
                params![request.tenant_id, request.tool_id, request.manifest_sha256],
            )
            .map_err(|_| StoreError::Database)?;
        if changed != 1 {
            transaction.commit().map_err(|_| StoreError::Database)?;
            return Err(StoreError::StaleDigest);
        }
        transaction.commit().map_err(|_| StoreError::Database)?;
        Ok(())
    }
}
