//! Local, fail-closed ingress for independently signed Architect research
//! evidence. This CLI does NOT perform Deep Research or execute OMAt tools.
//! The trusted service manager supplies tenant identity, public signer keys
//! and access to the local SQLite journals; never expose it to untrusted users.

use ocaf_runtime::{
    architect_research_evidence::{ResearchEvidence, ResearchEvidenceGate},
    architect_source_archive::SourceArchive,
    om_engine::OmEngine,
    omnidev_workflow::Completion,
};
use serde::Deserialize;
use std::{
    env,
    error::Error,
    io::{self, Read},
    path::PathBuf,
    process,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_INPUT_BYTES: u64 = 40 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Submission {
    evidence: ResearchEvidence,
    signature_hex: String,
}

struct Config {
    tenant: String,
    evidence_db: PathBuf,
    om_db: PathBuf,
    archive_db: Option<PathBuf>,
    planner_key: [u8; 32],
    architect_key: [u8; 32],
    verifier_key: [u8; 32],
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn required(name: &'static str) -> io::Result<String> {
    env::var(name).map_err(|_| invalid("required OCAF_EVIDENCE_* configuration missing"))
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}

fn path(name: &'static str) -> io::Result<PathBuf> {
    let p = PathBuf::from(required(name)?);
    if !p.is_absolute() || p.file_name().is_none() {
        return Err(invalid("database paths must be absolute files"));
    }
    Ok(p)
}

fn decode_hex(raw: &str, wanted_bytes: usize) -> io::Result<Vec<u8>> {
    if raw.len() != wanted_bytes * 2
        || !raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("invalid lowercase hexadecimal key or signature"));
    }
    let mut out = Vec::with_capacity(wanted_bytes);
    for part in raw.as_bytes().chunks(2) {
        let first = std::str::from_utf8(part).map_err(|_| invalid("invalid hex encoding"))?;
        out.push(u8::from_str_radix(first, 16).map_err(|_| invalid("invalid hex encoding"))?);
    }
    Ok(out)
}

/// Refuse sharing one physical SQLite journal across trust domains, even
/// through a symlink or a hard link. The service manager must also protect
/// parent directories against race-based path substitution.
fn distinct_archive(
    archive: &std::path::Path,
    om: &std::path::Path,
    evidence: &std::path::Path,
) -> io::Result<()> {
    for other in [om, evidence] {
        if archive == other {
            return Err(invalid(
                "source archive must be distinct from other journals",
            ));
        }
        if other.symlink_metadata().is_err() {
            continue;
        }
        if std::fs::canonicalize(archive)? == std::fs::canonicalize(other)? {
            return Err(invalid(
                "source archive must be distinct from other journals",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let a = std::fs::metadata(archive)?;
            let b = std::fs::metadata(other)?;
            if a.ino() == b.ino() && a.dev() == b.dev() {
                return Err(invalid(
                    "source archive must be distinct from other journals",
                ));
            }
        }
    }
    Ok(())
}

fn public_key(name: &'static str) -> io::Result<[u8; 32]> {
    let key = decode_hex(&required(name)?, 32)?;
    key.try_into()
        .map_err(|_| invalid("invalid Ed25519 public key"))
}

fn config() -> io::Result<Config> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let archived_mode = match arguments.as_slice() {
        [arg] if arg == "--submit" => false,
        [arg] if arg == "--submit-archived" => true,
        _ => return Err(invalid("only --submit and --submit-archived are supported")),
    };
    let tenant = required("OCAF_EVIDENCE_TENANT")?;
    if !valid_id(&tenant) {
        return Err(invalid("invalid trusted tenant identity"));
    }
    let evidence_db = path("OCAF_EVIDENCE_DB")?;
    let om_db = path("OCAF_EVIDENCE_OM_DB")?;
    if evidence_db == om_db {
        return Err(invalid("OM Engine and evidence journals must be separate"));
    }
    let archive_db = if archived_mode {
        let db = path("OCAF_EVIDENCE_ARCHIVE_DB")
            .map_err(|_| invalid("source archive must be configured for --submit-archived"))?;
        let metadata = std::fs::symlink_metadata(&db)
            .map_err(|_| invalid("source archive must already exist"))?;
        if !metadata.file_type().is_file() {
            return Err(invalid(
                "source archive must be an existing distinct regular file",
            ));
        }
        distinct_archive(&db, &om_db, &evidence_db)?;
        Some(db)
    } else {
        None
    };
    let planner_key = public_key("OCAF_EVIDENCE_PLANNER_PUBKEY_ED25519")?;
    let architect_key = public_key("OCAF_EVIDENCE_ARCHITECT_PUBKEY_ED25519")?;
    let verifier_key = public_key("OCAF_EVIDENCE_VERIFIER_PUBKEY_ED25519")?;
    if planner_key == architect_key || planner_key == verifier_key || architect_key == verifier_key
    {
        return Err(invalid("role-specific signer public keys must differ"));
    }
    Ok(Config {
        tenant,
        evidence_db,
        om_db,
        archive_db,
        planner_key,
        architect_key,
        verifier_key,
    })
}

fn run() -> Result<(), Box<dyn Error>> {
    let cfg = config()?;
    let mut raw = Vec::new();
    io::stdin()
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut raw)?;
    if raw.is_empty() || raw.len() as u64 > MAX_INPUT_BYTES {
        return Err(invalid("input exceeds bounded evidence envelope size").into());
    }
    let submission: Submission = serde_json::from_slice(&raw)?;
    let signature = decode_hex(&submission.signature_hex, 64)?;
    if submission.evidence.tenant != cfg.tenant {
        return Err(invalid("authenticated tenant does not match signed evidence").into());
    }
    // Both sources are protected local paths; this CLI cannot authenticate a
    // caller by itself. Filesystem/service-manager permissions must enforce
    // the tenant and role trust boundary.
    let gate = ResearchEvidenceGate::open(
        &cfg.evidence_db,
        &cfg.planner_key,
        &cfg.architect_key,
        &cfg.verifier_key,
    )?;
    let engine = OmEngine::open(&cfg.om_db)?;
    let now_unix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let result = if let Some(ref archive_db) = cfg.archive_db {
        let archive = SourceArchive::open(archive_db)?;
        gate.submit_with_archive(
            &engine,
            &archive,
            &cfg.tenant,
            &submission.evidence,
            &signature,
            now_unix,
        )?
    } else {
        gate.submit(
            &engine,
            &cfg.tenant,
            &submission.evidence,
            &signature,
            now_unix,
        )?
    };
    let label = match result.completion {
        Completion::Recorded => "RECORDED",
        Completion::Replay => "REPLAY",
    };
    println!(
        "OCAF_ARCHITECT_EVIDENCE {label} sha256:{}",
        result.digest_sha256
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("evidence_rejected: {error}");
        process::exit(1);
    }
}
