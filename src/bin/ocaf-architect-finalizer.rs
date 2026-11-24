//! OCAF Meta Agent Architect signed research finalization process.
//!
//! One-shot verified reconciliation. It does not execute research, authorize
//! release, or make network calls. The service manager must authenticate the
//! tenant/fleet and supply separately pinned Ed25519 public keys and protected
//! journal file paths.

use ocaf_runtime::{
    architect_research_evidence::ResearchEvidenceGate,
    architect_research_finalizer::finalize_ready,
    architect_research_schedule::ArchitectResearchScheduler, om_engine::OmEngine,
};
use std::{
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

struct Config {
    tenant: String,
    fleet: String,
    scheduler_db: PathBuf,
    om_db: PathBuf,
    evidence_db: PathBuf,
    planner_pubkey: [u8; 32],
    architect_pubkey: [u8; 32],
    verifier_pubkey: [u8; 32],
    limit: usize,
}

fn invalid(description: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, description)
}

fn required(name: &'static str) -> io::Result<String> {
    env::var(name).map_err(|_| invalid("required OCAF_FINALIZER_* setting missing"))
}

fn valid_id(input: &str) -> bool {
    !input.is_empty()
        && input.len() <= 96
        && input
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(&byte))
}

fn existing_file(name: &'static str) -> io::Result<PathBuf> {
    let path = PathBuf::from(required(name)?);
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(invalid("journal path must be an absolute file"));
    }
    let meta = fs::symlink_metadata(&path)?;
    if !meta.file_type().is_file() {
        return Err(invalid("journal path is not an existing regular file"));
    }
    Ok(path)
}

fn decode_ed25519_key(name: &'static str) -> io::Result<[u8; 32]> {
    let raw = required(name)?;
    if raw.len() != 64
        || !raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("invalid Ed25519 public key encoding"));
    }
    let mut decoded = [0_u8; 32];
    for (idx, pair) in raw.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let text = std::str::from_utf8(pair).map_err(|_| invalid("invalid hex"))?;
        decoded[idx] = u8::from_str_radix(text, 16).map_err(|_| invalid("invalid hex"))?;
    }
    Ok(decoded)
}

fn validate_distinct_journals(paths: [&Path; 3]) -> io::Result<()> {
    let canonical: Vec<_> = paths
        .iter()
        .map(fs::canonicalize)
        .collect::<io::Result<Vec<_>>>()?;
    if canonical[0] == canonical[1] || canonical[0] == canonical[2] || canonical[1] == canonical[2]
    {
        return Err(invalid("journal files must be distinct"));
    }
    // Canonical paths do not identify multiple hard links to the same inode.
    // Independent SQLite journals must not share physical database files.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata: Vec<_> = paths
            .iter()
            .map(fs::metadata)
            .collect::<io::Result<Vec<_>>>()?;
        for i in 0..3 {
            for j in (i + 1)..3 {
                if metadata[i].dev() == metadata[j].dev() && metadata[i].ino() == metadata[j].ino()
                {
                    return Err(invalid("journal files must be distinct"));
                }
            }
        }
    }
    Ok(())
}

fn config() -> io::Result<Config> {
    if env::args().skip(1).collect::<Vec<_>>() != ["--once"] {
        return Err(invalid("only --once is supported"));
    }
    let tenant = required("OCAF_FINALIZER_TENANT")?;
    let fleet = required("OCAF_FINALIZER_FLEET")?;
    if !valid_id(&tenant) || !valid_id(&fleet) {
        return Err(invalid("invalid trusted identity"));
    }
    let planner_pubkey = decode_ed25519_key("OCAF_FINALIZER_PLANNER_PUBKEY_ED25519")?;
    let architect_pubkey = decode_ed25519_key("OCAF_FINALIZER_ARCHITECT_PUBKEY_ED25519")?;
    let verifier_pubkey = decode_ed25519_key("OCAF_FINALIZER_VERIFIER_PUBKEY_ED25519")?;
    if planner_pubkey == architect_pubkey
        || planner_pubkey == verifier_pubkey
        || architect_pubkey == verifier_pubkey
    {
        return Err(invalid("signer roles must use distinct public keys"));
    }
    let limit = match env::var("OCAF_FINALIZER_LIMIT") {
        Ok(raw) => raw.parse::<usize>().map_err(|_| invalid("invalid limit"))?,
        Err(env::VarError::NotPresent) => 64,
        Err(_) => return Err(invalid("invalid limit")),
    };
    if !(1..=64).contains(&limit) {
        return Err(invalid("limit must be in 1..=64"));
    }
    let scheduler_db = existing_file("OCAF_FINALIZER_SCHEDULE_DB")?;
    let om_db = existing_file("OCAF_FINALIZER_OM_DB")?;
    let evidence_db = existing_file("OCAF_FINALIZER_EVIDENCE_DB")?;
    validate_distinct_journals([&scheduler_db, &om_db, &evidence_db])?;
    Ok(Config {
        tenant,
        fleet,
        scheduler_db,
        om_db,
        evidence_db,
        planner_pubkey,
        architect_pubkey,
        verifier_pubkey,
        limit,
    })
}

fn run() -> Result<(), Box<dyn Error>> {
    let cfg = config()?;
    let now_unix = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())?;
    let scheduler = ArchitectResearchScheduler::open(&cfg.scheduler_db)?;
    let engine = OmEngine::open(&cfg.om_db)?;
    let evidence = ResearchEvidenceGate::open(
        &cfg.evidence_db,
        &cfg.planner_pubkey,
        &cfg.architect_pubkey,
        &cfg.verifier_pubkey,
    )?;
    let outcome = finalize_ready(
        &scheduler,
        &engine,
        &evidence,
        &cfg.tenant,
        &cfg.fleet,
        now_unix,
        cfg.limit,
    )?;
    println!(
        "OCAF_ARCHITECT_FINALIZER scanned={} finalized={} pending={}",
        outcome.scanned, outcome.finalized, outcome.pending
    );
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("research_finalizer_failed: {err}");
        process::exit(1);
    }
}
