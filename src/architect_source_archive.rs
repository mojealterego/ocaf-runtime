//! Bounded tenant-scoped SQLite archive of actual HTTP response bodies.
//!
//! Stored bytes are hashed before ingestion and again at every read. A source
//! reference without a matching, readable body is NOT sufficient evidence.
//! The trusted host must authenticate the tenant and must ensure that only
//! operator-controlled HTTPS fetch results enter this archive. Hash consistency
//! alone does not prove that the response originated at the declared URL.
//! SQLite must be externally anchored against privileged rollback/tampering.

use crate::{
    architect_https_source_fetch::{
        fetch_https, FetchedSource, PinnedSourceRoute, SourceFetchError,
    },
    architect_research_evidence::SourceRecord,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::{net::IpAddr, path::Path, sync::Mutex, time::Duration};
use thiserror::Error;
use url::Url;

const MAX_BODY_BYTES: usize = 1_048_576;
const MAX_UNIX: u64 = 253_402_300_799;
const MAX_ROWS_PER_TENANT: i64 = 4096;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SaveResult {
    New,
    Replay,
}

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("invalid source, content digest or tenant identity")]
    Invalid,
    #[error("source record missing from tenant archive")]
    Missing,
    #[error("immutable source version already exists with different contents")]
    Conflict,
    #[error("archived source bytes or metadata failed integrity verification")]
    Integrity,
    #[error("tenant source retention limit reached")]
    Capacity,
    #[error("source archive database lock poisoned")]
    Poisoned,
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Fetch(#[from] SourceFetchError),
}

pub struct SourceArchive {
    db: Mutex<Connection>,
}

fn valid_id(input: &str) -> bool {
    !input.is_empty()
        && input.len() <= 96
        && input
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}

fn valid_sha(input: &str) -> bool {
    input.len() == 64
        && input
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn validate_ref(source: &SourceRecord) -> Result<(), ArchiveError> {
    if source.url.is_empty()
        || source.url.len() > 2048
        || source.retrieved_unix == 0
        || source.retrieved_unix > MAX_UNIX
        || !valid_sha(&source.content_sha256)
    {
        return Err(ArchiveError::Invalid);
    }
    let url = Url::parse(&source.url).map_err(|_| ArchiveError::Invalid)?;
    let host = url.host_str().ok_or(ArchiveError::Invalid)?;
    // Keep this stricter than an arbitrary URL parser: no credentials, IP,
    // internal hostnames, fragments or ambiguous spelling.
    if url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !host.contains('.')
        || host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".local")
        || host.parse::<IpAddr>().is_ok()
    {
        return Err(ArchiveError::Invalid);
    }
    Ok(())
}

fn digest(body: &[u8]) -> String {
    format!("{:x}", Sha256::digest(body))
}

fn validate_bytes(body: &[u8]) -> Result<(), ArchiveError> {
    if !(1..=MAX_BODY_BYTES).contains(&body.len())
        || body.contains(&0)
        || std::str::from_utf8(body).is_err()
    {
        return Err(ArchiveError::Invalid);
    }
    Ok(())
}

impl SourceArchive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ArchiveError> {
        if path
            .as_ref()
            .symlink_metadata()
            .is_ok_and(|meta| meta.file_type().is_symlink())
        {
            return Err(ArchiveError::Invalid);
        }
        let db = Connection::open(path)?;
        db.busy_timeout(Duration::from_secs(5))?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON;
             PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS architect_source_archive (
                tenant TEXT NOT NULL,
                source_url TEXT NOT NULL,
                retrieved_unix INTEGER NOT NULL,
                content_sha256 TEXT NOT NULL,
                body BLOB NOT NULL,
                PRIMARY KEY(tenant,source_url,retrieved_unix)
             );
             CREATE INDEX IF NOT EXISTS idx_archived_source_tenant
             ON architect_source_archive(tenant);",
        )?;
        Ok(Self { db: Mutex::new(db) })
    }

    /// Ingest an authenticated host-supplied fetched response. The body must
    /// match its SHA-256 before any bytes can be committed. The caller's host
    /// authorization is external; this is not an agent-accessible ingestion API.
    pub fn save(
        &self,
        trusted_tenant: &str,
        fetched: &FetchedSource,
    ) -> Result<SaveResult, ArchiveError> {
        if !valid_id(trusted_tenant) || fetched.status != 200 {
            return Err(ArchiveError::Invalid);
        }
        validate_ref(&fetched.source)?;
        validate_bytes(&fetched.body)?;
        if digest(&fetched.body) != fetched.source.content_sha256 {
            return Err(ArchiveError::Invalid);
        }
        let mut db = self.db.lock().map_err(|_| ArchiveError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior: Option<(String, Vec<u8>)> = tx
            .query_row(
                "SELECT content_sha256,body FROM architect_source_archive
                 WHERE tenant=?1 AND source_url=?2 AND retrieved_unix=?3",
                params![
                    trusted_tenant,
                    fetched.source.url,
                    fetched.source.retrieved_unix
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((prior_hash, prior_body)) = prior {
            if digest(&prior_body) != prior_hash {
                return Err(ArchiveError::Integrity);
            }
            if prior_hash != fetched.source.content_sha256 || prior_body != fetched.body {
                return Err(ArchiveError::Conflict);
            }
            tx.commit()?;
            return Ok(SaveResult::Replay);
        }
        let count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM architect_source_archive WHERE tenant=?1",
            params![trusted_tenant],
            |row| row.get(0),
        )?;
        if count >= MAX_ROWS_PER_TENANT {
            return Err(ArchiveError::Capacity);
        }
        tx.execute(
            "INSERT INTO architect_source_archive
             (tenant,source_url,retrieved_unix,content_sha256,body)
             VALUES (?1,?2,?3,?4,?5)",
            params![
                trusted_tenant,
                fetched.source.url,
                fetched.source.retrieved_unix,
                fetched.source.content_sha256,
                fetched.body
            ],
        )?;
        tx.commit()?;
        Ok(SaveResult::New)
    }

    /// Perform the actual operator-pinned TLS GET and store exactly its body.
    /// A successful return means both HTTPS fetching and the SQLite write
    /// completed. This method never receives arbitrary user-supplied bytes.
    pub fn fetch_and_store(
        &self,
        trusted_tenant: &str,
        route: &PinnedSourceRoute,
        now_unix: u64,
    ) -> Result<(SourceRecord, SaveResult), ArchiveError> {
        if !valid_id(trusted_tenant) || route.tenant != trusted_tenant {
            return Err(ArchiveError::Invalid);
        }
        let fetched = fetch_https(route, trusted_tenant, now_unix)?;
        let saved = self.save(trusted_tenant, &fetched)?;
        Ok((fetched.source, saved))
    }

    /// Read the exact archived bytes, verifying their stored digest and the
    /// caller-provided signed source reference. Never substitutes missing data
    /// or returns source contents from a different tenant.
    pub fn read(
        &self,
        trusted_tenant: &str,
        source: &SourceRecord,
    ) -> Result<Vec<u8>, ArchiveError> {
        if !valid_id(trusted_tenant) {
            return Err(ArchiveError::Invalid);
        }
        validate_ref(source)?;
        let db = self.db.lock().map_err(|_| ArchiveError::Poisoned)?;
        let existing: Option<(String, Vec<u8>)> = db
            .query_row(
                "SELECT content_sha256,body FROM architect_source_archive
                 WHERE tenant=?1 AND source_url=?2 AND retrieved_unix=?3",
                params![trusted_tenant, source.url, source.retrieved_unix],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (stored_hash, body) = existing.ok_or(ArchiveError::Missing)?;
        if body.len() > MAX_BODY_BYTES
            || stored_hash != source.content_sha256
            || digest(&body) != stored_hash
        {
            return Err(ArchiveError::Integrity);
        }
        Ok(body)
    }

    /// Verify all source references against retained actual response bodies.
    /// A zero-source plan or verifier step is permitted; the separate evidence
    /// gate enforces >=2 references on a research step.
    pub fn verify_sources(
        &self,
        trusted_tenant: &str,
        sources: &[SourceRecord],
    ) -> Result<(), ArchiveError> {
        if sources.len() > 32 {
            return Err(ArchiveError::Invalid);
        }
        for source in sources {
            self.read(trusted_tenant, source)?;
        }
        Ok(())
    }
}
