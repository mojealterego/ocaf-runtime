use ocaf_runtime::{
    architect_https_source_fetch::FetchedSource,
    architect_research_evidence::{ResearchEvidence, ResearchEvidenceGate, SourceRecord},
    architect_source_archive::SourceArchive,
    om_engine::OmEngine,
    omnidev_workflow::{Outcome, Role, StepContract, StepStatus, WorkflowContract},
};
use openssl::{
    pkey::{PKey, Private},
    sign::Signer,
};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::tempdir;

fn sha(body: &[u8]) -> String {
    format!("{:x}", Sha256::digest(body))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn keys() -> [PKey<Private>; 3] {
    [
        PKey::generate_ed25519().unwrap(),
        PKey::generate_ed25519().unwrap(),
        PKey::generate_ed25519().unwrap(),
    ]
}
fn fetched(name: &str, now: u64) -> FetchedSource {
    let body = format!("Retained content: {name}").into_bytes();
    FetchedSource {
        source: SourceRecord {
            url: format!("https://example.com/{}", name),
            retrieved_unix: now - 20,
            content_sha256: sha(&body),
        },
        status: 200,
        body,
    }
}
fn execute(
    om: &Path,
    evidence_db: &Path,
    archive_db: &Path,
    keys: &[PKey<Private>; 3],
    report: &ResearchEvidence,
) -> std::process::Output {
    let signature = Signer::new_without_digest(&keys[1])
        .unwrap()
        .sign_oneshot_to_vec(&report.signing_message().unwrap())
        .unwrap();
    let input = serde_json::json!({"evidence":report,"signature_hex":hex(&signature)}).to_string();
    let mut process = Command::new(env!("CARGO_BIN_EXE_ocaf-architect-evidence"))
        .arg("--submit-archived")
        .env("OCAF_EVIDENCE_TENANT", "t1")
        .env("OCAF_EVIDENCE_OM_DB", om)
        .env("OCAF_EVIDENCE_DB", evidence_db)
        .env("OCAF_EVIDENCE_ARCHIVE_DB", archive_db)
        .env(
            "OCAF_EVIDENCE_PLANNER_PUBKEY_ED25519",
            hex(&keys[0].raw_public_key().unwrap()),
        )
        .env(
            "OCAF_EVIDENCE_ARCHITECT_PUBKEY_ED25519",
            hex(&keys[1].raw_public_key().unwrap()),
        )
        .env(
            "OCAF_EVIDENCE_VERIFIER_PUBKEY_ED25519",
            hex(&keys[2].raw_public_key().unwrap()),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    process.wait_with_output().unwrap()
}

#[test]
fn real_evidence_binary_requires_retained_sources_and_rechecks_on_replay() {
    let dir = tempdir().unwrap();
    let om = dir.path().join("engine.sqlite");
    let proof = dir.path().join("evidence.sqlite");
    let archive_db = dir.path().join("archive.sqlite");
    let engine = OmEngine::open(&om).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    engine
        .register(
            "t1",
            WorkflowContract {
                tenant: "t1".into(),
                workflow_id: "bound-sources".into(),
                base_commit_sha: "c".repeat(40),
                deadline_unix: now + 300,
                max_cost_minor: 30,
                steps: vec![StepContract {
                    id: "research".into(),
                    role: Role::Architect,
                    prerequisites: vec![],
                    max_attempts: 1,
                    max_cost_minor: 30,
                    lease_seconds: 300,
                }],
            },
        )
        .unwrap();
    engine
        .claim(
            "t1",
            "bound-sources",
            Role::Architect,
            "research",
            1,
            "attempt",
            now,
        )
        .unwrap();
    let archives = SourceArchive::open(&archive_db).unwrap();
    let one = fetched("one", now);
    let two = fetched("two", now);
    let report = ResearchEvidence {
        schema_version: 1,
        tenant: "t1".into(),
        workflow_id: "bound-sources".into(),
        step: "research".into(),
        role: Role::Architect,
        epoch: 1,
        attempt_id: "attempt".into(),
        observed_at_unix: now,
        actual_cost_minor: 3,
        outcome: Outcome::Passed,
        summary: "Two independently retained sources are required before signed execution passes"
            .into(),
        sources: vec![one.source.clone(), two.source.clone()],
        review_of_sha256: None,
    };
    let keys = keys();
    let missing = execute(&om, &proof, &archive_db, &keys, &report);
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("source record missing"),
        "missing error: {:?}",
        String::from_utf8_lossy(&missing.stderr)
    );
    assert!(matches!(
        engine.status("t1", "bound-sources", "research"),
        Ok(StepStatus::Leased(_))
    ));
    archives.save("t2", &one).unwrap(); // different tenant cannot supply source
    archives.save("t1", &two).unwrap();
    assert!(!execute(&om, &proof, &archive_db, &keys, &report)
        .status
        .success());
    archives.save("t1", &one).unwrap();
    let success = execute(&om, &proof, &archive_db, &keys, &report);
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    assert!(String::from_utf8_lossy(&success.stdout).contains("RECORDED"));
    let replay = execute(&om, &proof, &archive_db, &keys, &report);
    assert!(
        replay.status.success(),
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
    assert!(String::from_utf8_lossy(&replay.stdout).contains("REPLAY"));
    let conn = rusqlite::Connection::open(&archive_db).unwrap();
    conn.execute(
        "UPDATE architect_source_archive SET body=X'6861636b' WHERE tenant='t1' AND source_url=?1",
        rusqlite::params![one.source.url],
    )
    .unwrap();
    let tampered = execute(&om, &proof, &archive_db, &keys, &report);
    assert!(!tampered.status.success());
    assert!(
        String::from_utf8_lossy(&tampered.stderr).contains("integrity"),
        "unexpected: {:?}",
        String::from_utf8_lossy(&tampered.stderr)
    );
    let gate = ResearchEvidenceGate::open(
        &proof,
        &keys[0].raw_public_key().unwrap(),
        &keys[1].raw_public_key().unwrap(),
        &keys[2].raw_public_key().unwrap(),
    )
    .unwrap();
    assert!(gate
        .read("t1", "bound-sources", "research", 1, "attempt")
        .is_ok());
}
