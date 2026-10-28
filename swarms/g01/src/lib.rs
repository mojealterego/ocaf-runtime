//! G01 local engineering handoff. Claims and completions are replayed from an
//! atomically replaced, fsynced journal; completion requires a pinned Ed25519
//! operator signature and the bytes of the artifact named by its SHA-256.

pub mod zenoh;

use fs2::FileExt;
use ocaf_stage_ledger::{StageLedger, StageLedgerError, StageReceipt, StageRecord, StageStatus};
use openssl::pkey::{Id, PKey};
use openssl::sign::{Signer, Verifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

pub const SWARM_ID: &str = "OCAF-SWARM-01";
const JOURNAL_VERSION: u32 = 1;
const RECEIPT_DOMAIN: &[u8] = b"OCAF-G01-OPERATOR-RECEIPT-v1\0";
const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("stage ledger: {0}")]
    Ledger(#[from] StageLedgerError),
    #[error("cryptography: {0}")]
    Crypto(#[from] openssl::error::ErrorStack),
    #[error("rejected: {0}")]
    Rejected(String),
}

type Result<T> = std::result::Result<T, Error>;

fn reject(message: impl Into<String>) -> Error {
    Error::Rejected(message.into())
}

/// An explicit local-development request. Unrecognized fields and all ACTIVE,
/// readiness, OSTc, and source-access claims are rejected before a stage claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimRequest {
    pub swarm_id: String,
    pub declared_state: String,
    pub readiness: bool,
    pub ostc_enabled: bool,
    pub source_access: String,
}

impl ClaimRequest {
    fn validate(&self) -> Result<()> {
        if self.swarm_id != SWARM_ID {
            return Err(reject("wrong swarm identity"));
        }
        if self.declared_state != "IN_DEVELOPMENT"
            || self.readiness
            || self.ostc_enabled
            || self.source_access != "disabled"
        {
            return Err(reject("ACTIVE/readiness/OSTc/source access denied"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptPayload {
    pub version: u32,
    pub store_id: String,
    pub receipt: StageReceipt,
}

impl ReceiptPayload {
    /// Stable, domain-separated signature message. JSON field order on disk
    /// does not matter because the struct is serialized in a fixed order.
    pub fn signature_message(&self) -> Result<Vec<u8>> {
        let mut message = RECEIPT_DOMAIN.to_vec();
        message.extend_from_slice(&serde_json::to_vec(self)?);
        Ok(message)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedReceipt {
    pub payload: ReceiptPayload,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Event {
    Claim {
        stage_id: String,
        claimed_by: String,
    },
    Complete {
        signed: SignedReceipt,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u32,
    store_id: String,
    operator_public_key_hex: String,
    plan: Vec<StageRecord>,
    events: Vec<Event>,
}

pub struct SwarmStore {
    root: PathBuf,
}

impl SwarmStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Pin a local plan and operator public key. No stage is claimed at init.
    pub fn initialize(&self, plan: Vec<StageRecord>, operator_public_key_hex: &str) -> Result<()> {
        if plan.is_empty()
            || plan
                .iter()
                .any(|stage| stage.status != StageStatus::Ready || stage.claimed_by.is_some())
        {
            return Err(reject(
                "initial plan must contain only unclaimed READY stages",
            ));
        }
        StageLedger::new(plan.clone())?;
        let public = decode_hex::<32>(operator_public_key_hex)?;
        PKey::public_key_from_raw_bytes(&public, Id::ED25519)?;
        fs::create_dir_all(&self.root)?;
        let _lock = self.lock()?;
        if self.journal_path().exists() {
            return Err(reject("journal already exists"));
        }
        fs::create_dir_all(self.root.join("artifacts"))?;
        let journal = Journal {
            version: JOURNAL_VERSION,
            store_id: Uuid::new_v4().to_string(),
            operator_public_key_hex: encode_hex(&public),
            plan,
            events: Vec::new(),
        };
        self.write_journal(&journal)
    }

    pub fn status(&self) -> Result<Vec<StageRecord>> {
        let _lock = self.lock()?;
        let (_, ledger) = self.load()?;
        Ok(ledger.statuses().cloned().collect())
    }

    pub fn next(&self) -> Result<Option<StageRecord>> {
        let _lock = self.lock()?;
        let (_, ledger) = self.load()?;
        if ledger
            .statuses()
            .any(|stage| stage.status == StageStatus::Claimed)
        {
            return Ok(None);
        }
        Ok(ledger.next_eligible().cloned())
    }

    pub fn claim(&self, request: ClaimRequest) -> Result<String> {
        request.validate()?;
        let _lock = self.lock()?;
        let (mut journal, mut ledger) = self.load()?;
        if ledger
            .statuses()
            .any(|stage| stage.status == StageStatus::Claimed)
        {
            return Err(reject("an earlier claim is still open"));
        }
        let stage_id = ledger.claim_next(SWARM_ID)?;
        journal.events.push(Event::Claim {
            stage_id: stage_id.clone(),
            claimed_by: SWARM_ID.into(),
        });
        self.write_journal(&journal)?;
        Ok(stage_id)
    }

    /// Hash the supplied file now and form the exact body an operator reviews.
    /// Handoff hashes and copies it again before committing the signed event.
    pub fn prepare(
        &self,
        artifact: &Path,
        source_commit: String,
        test_command: String,
        test_result: String,
        receipt_id: String,
        known_limits: Vec<String>,
    ) -> Result<ReceiptPayload> {
        let _lock = self.lock()?;
        let (journal, ledger) = self.load()?;
        let stage = open_claim(&ledger)?;
        let receipt = StageReceipt {
            stage_id: stage.stage_id.clone(),
            claimed_by: SWARM_ID.into(),
            source_commit,
            test_command,
            test_result,
            artifact_digest: format!("sha256:{}", hash_file(artifact)?),
            receipt_id,
            known_limits,
        };
        let mut preview = ledger;
        preview.complete(receipt.clone())?;
        Ok(ReceiptPayload {
            version: JOURNAL_VERSION,
            store_id: journal.store_id,
            receipt,
        })
    }

    /// Optional local operator helper. The private key is supplied by the
    /// operator and is never persisted. The artifact is rehashed before signing.
    pub fn operator_sign(
        &self,
        artifact: &Path,
        payload: ReceiptPayload,
        private_key: &[u8; 32],
    ) -> Result<SignedReceipt> {
        let _lock = self.lock()?;
        let (journal, mut ledger) = self.load()?;
        validate_payload(&journal, &payload)?;
        if payload.receipt.artifact_digest != format!("sha256:{}", hash_file(artifact)?) {
            return Err(reject("artifact SHA-256 differs from receipt"));
        }
        ledger.complete(payload.receipt.clone())?;
        let key = PKey::private_key_from_raw_bytes(private_key, Id::ED25519)?;
        if encode_hex(&key.raw_public_key()?) != journal.operator_public_key_hex {
            return Err(reject("operator private key is not pinned by this store"));
        }
        let signature =
            Signer::new_without_digest(&key)?.sign_oneshot_to_vec(&payload.signature_message()?)?;
        Ok(SignedReceipt {
            payload,
            signature_hex: encode_hex(&signature),
        })
    }

    /// Append a completion only after signature and source artifact checks.
    /// A copy is stored by digest, so later journal replay detects loss or
    /// alteration of completed artifacts.
    pub fn handoff(&self, artifact: &Path, signed: SignedReceipt) -> Result<String> {
        let _lock = self.lock()?;
        let (mut journal, mut ledger) = self.load()?;
        verify_signed(&journal, &signed)?;
        let stage_id = signed.payload.receipt.stage_id.clone();
        ledger.complete(signed.payload.receipt.clone())?;
        self.store_artifact(artifact, &signed.payload.receipt.artifact_digest)?;
        journal.events.push(Event::Complete { signed });
        self.write_journal(&journal)?;
        Ok(stage_id)
    }

    fn journal_path(&self) -> PathBuf {
        self.root.join("journal.json")
    }

    fn lock(&self) -> Result<File> {
        if !self.root.is_dir() {
            return Err(reject("store directory does not exist"));
        }
        let path = self.root.join(".lock");
        if fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err(reject("symlinked lock file"));
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)?;
        file.lock_exclusive()?;
        Ok(file)
    }

    fn load(&self) -> Result<(Journal, StageLedger)> {
        let path = self.journal_path();
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(reject("symlinked journal"));
        }
        if fs::metadata(&path)?.len() > 16 * 1024 * 1024 {
            return Err(reject("journal is too large"));
        }
        let journal: Journal = serde_json::from_slice(&fs::read(path)?)?;
        if journal.version != JOURNAL_VERSION || Uuid::parse_str(&journal.store_id).is_err() {
            return Err(reject("unsupported journal version or store id"));
        }
        let key = decode_hex::<32>(&journal.operator_public_key_hex)?;
        PKey::public_key_from_raw_bytes(&key, Id::ED25519)?;
        if journal.plan.is_empty()
            || journal
                .plan
                .iter()
                .any(|stage| stage.status != StageStatus::Ready || stage.claimed_by.is_some())
        {
            return Err(reject("invalid initial plan"));
        }
        let mut ledger = StageLedger::new(journal.plan.clone())?;
        for event in &journal.events {
            match event {
                Event::Claim {
                    stage_id,
                    claimed_by,
                } => {
                    if claimed_by != SWARM_ID
                        || ledger
                            .statuses()
                            .any(|stage| stage.status == StageStatus::Claimed)
                        || ledger.claim_next(claimed_by)? != *stage_id
                    {
                        return Err(reject("journal contains an out-of-order claim"));
                    }
                }
                Event::Complete { signed } => {
                    verify_signed(&journal, signed)?;
                    self.verify_stored_artifact(&signed.payload.receipt.artifact_digest)?;
                    ledger.complete(signed.payload.receipt.clone())?;
                }
            }
        }
        Ok((journal, ledger))
    }

    fn write_journal(&self, journal: &Journal) -> Result<()> {
        let temp = self.root.join(format!(".journal-{}.tmp", Uuid::new_v4()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        serde_json::to_writer(&mut file, journal)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::rename(&temp, self.journal_path())?;
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }

    fn verify_stored_artifact(&self, digest: &str) -> Result<()> {
        let hex = digest_hex(digest)?;
        let path = self.root.join("artifacts").join(format!("{hex}.bin"));
        if fs::symlink_metadata(&path)?.file_type().is_symlink() || hash_file(&path)? != hex {
            return Err(reject("stored artifact is missing or altered"));
        }
        Ok(())
    }

    fn store_artifact(&self, source: &Path, digest: &str) -> Result<()> {
        let expected = digest_hex(digest)?;
        let dir = self.root.join("artifacts");
        fs::create_dir_all(&dir)?;
        let target = dir.join(format!("{expected}.bin"));
        if target.exists() {
            if hash_file(source)? != expected {
                return Err(reject("artifact SHA-256 differs from signed receipt"));
            }
            return self.verify_stored_artifact(digest);
        }
        let temp = dir.join(format!(".artifact-{}.tmp", Uuid::new_v4()));
        let mut input = File::open(source)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let mut hasher = Sha256::new();
        let mut bytes = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        let copied = (|| -> Result<()> {
            loop {
                let count = input.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                bytes += count as u64;
                if bytes > MAX_ARTIFACT_BYTES {
                    return Err(reject("artifact exceeds 64 MiB"));
                }
                hasher.update(&buffer[..count]);
                output.write_all(&buffer[..count])?;
            }
            output.sync_all()?;
            if encode_hex(&hasher.finalize()) != expected {
                return Err(reject("artifact SHA-256 differs from signed receipt"));
            }
            Ok(())
        })();
        if let Err(error) = copied {
            let _ = fs::remove_file(temp);
            return Err(error);
        }
        fs::rename(&temp, &target)?;
        File::open(dir)?.sync_all()?;
        Ok(())
    }
}

fn open_claim(ledger: &StageLedger) -> Result<&StageRecord> {
    ledger
        .statuses()
        .find(|stage| {
            stage.status == StageStatus::Claimed && stage.claimed_by.as_deref() == Some(SWARM_ID)
        })
        .ok_or_else(|| reject("G01 has no open claim"))
}

fn validate_payload(journal: &Journal, payload: &ReceiptPayload) -> Result<()> {
    if payload.version != JOURNAL_VERSION || payload.store_id != journal.store_id {
        return Err(reject("receipt version or store id mismatch"));
    }
    Ok(())
}

fn verify_signed(journal: &Journal, signed: &SignedReceipt) -> Result<()> {
    validate_payload(journal, &signed.payload)?;
    let public = decode_hex::<32>(&journal.operator_public_key_hex)?;
    let signature = decode_hex::<64>(&signed.signature_hex)?;
    let key = PKey::public_key_from_raw_bytes(&public, Id::ED25519)?;
    if !Verifier::new_without_digest(&key)?
        .verify_oneshot(&signature, &signed.payload.signature_message()?)?
    {
        return Err(reject("operator Ed25519 signature rejected"));
    }
    Ok(())
}

fn digest_hex(digest: &str) -> Result<String> {
    let value = digest
        .strip_prefix("sha256:")
        .ok_or_else(|| reject("artifact digest must use sha256"))?;
    let bytes = decode_hex::<32>(value)?;
    Ok(encode_hex(&bytes))
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    if file.metadata()?.len() > MAX_ARTIFACT_BYTES {
        return Err(reject("artifact exceeds 64 MiB"));
    }
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > MAX_ARTIFACT_BYTES {
            return Err(reject("artifact exceeds 64 MiB"));
        }
        hasher.update(&buffer[..count]);
    }
    Ok(encode_hex(&hasher.finalize()))
}

pub fn decode_hex<const N: usize>(hex: &str) -> Result<[u8; N]> {
    if hex.len() != N * 2 || !hex.is_ascii() {
        return Err(reject(format!("expected {} hex bytes", N)));
    }
    let mut output = [0_u8; N];
    for (index, slot) in output.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .map_err(|_| reject("invalid hex"))?;
    }
    Ok(output)
}

pub fn encode_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 15) as usize] as char);
    }
    output
}
