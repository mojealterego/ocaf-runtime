use ocaf_runtime::{
    architect_research_evidence::{ResearchEvidence, ResearchEvidenceGate, SourceRecord},
    om_engine::OmEngine,
    omnidev_workflow::{Outcome, Role, StepContract, StepStatus, WorkflowContract},
};
use openssl::{
    pkey::{PKey, Private},
    sign::Signer,
};
use serde_json::json;
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::tempdir;

fn key() -> PKey<Private> {
    PKey::generate_ed25519().unwrap()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn contract() -> WorkflowContract {
    WorkflowContract {
        tenant: "tenant_a".into(),
        workflow_id: "research_01".into(),
        base_commit_sha: "a".repeat(40),
        deadline_unix: u64::MAX - 100,
        max_cost_minor: 120,
        steps: vec![StepContract {
            id: "plan".into(),
            role: Role::Planner,
            prerequisites: vec![],
            max_attempts: 1,
            max_cost_minor: 40,
            lease_seconds: 300,
        }],
    }
}
fn report(now: u64) -> ResearchEvidence {
    ResearchEvidence {
        schema_version: 1,
        tenant: "tenant_a".into(),
        workflow_id: "research_01".into(),
        step: "plan".into(),
        role: Role::Planner,
        epoch: 1,
        attempt_id: "p1".into(),
        observed_at_unix: now,
        actual_cost_minor: 4,
        outcome: Outcome::Passed,
        summary: "Plan established for external research, awaiting independent data".into(),
        sources: vec![],
        review_of_sha256: None,
    }
}
fn run(
    evidence_path: &Path,
    engine_path: &Path,
    keys: &[PKey<Private>; 3],
    body: &[u8],
    tenant: &str,
) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ocaf-architect-evidence"))
        .arg("--submit")
        .env("OCAF_EVIDENCE_DB", evidence_path)
        .env("OCAF_EVIDENCE_OM_DB", engine_path)
        .env("OCAF_EVIDENCE_TENANT", tenant)
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
    let write_result = child.stdin.take().unwrap().write_all(body);
    if let Err(error) = write_result {
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
    child.wait_with_output().unwrap()
}
fn envelope(evidence: &ResearchEvidence, key: &PKey<Private>) -> Vec<u8> {
    let message = evidence.signing_message().unwrap();
    let signature = Signer::new_without_digest(key)
        .unwrap()
        .sign_oneshot_to_vec(&message)
        .unwrap();
    serde_json::to_vec(&json!({"evidence":evidence,"signature_hex":hex(&signature)})).unwrap()
}

#[test]
fn actual_cli_submits_signed_plan_to_real_om_engine_and_replays() {
    let dir = tempdir().unwrap();
    let engine_path = dir.path().join("om.db");
    let evidence_path = dir.path().join("proofs.db");
    let engine = OmEngine::open(&engine_path).unwrap();
    engine.register("tenant_a", contract()).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    engine
        .claim(
            "tenant_a",
            "research_01",
            Role::Planner,
            "plan",
            1,
            "p1",
            now,
        )
        .unwrap();
    let keys = [key(), key(), key()];
    let evidence = report(now);
    let bytes = envelope(&evidence, &keys[0]);
    let first = run(&evidence_path, &engine_path, &keys, &bytes, "tenant_a");
    assert!(
        first.status.success(),
        "{:?}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(String::from_utf8_lossy(&first.stdout).contains("RECORDED"));
    assert!(matches!(
        engine.status("tenant_a", "research_01", "plan").unwrap(),
        StepStatus::Passed { .. }
    ));
    let second = run(&evidence_path, &engine_path, &keys, &bytes, "tenant_a");
    assert!(
        second.status.success(),
        "{:?}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(String::from_utf8_lossy(&second.stdout).contains("REPLAY"));
    assert_eq!(
        engine.audit_head("tenant_a", "research_01").unwrap().events,
        2
    );
    let store = ResearchEvidenceGate::open(
        &evidence_path,
        &keys[0].raw_public_key().unwrap(),
        &keys[1].raw_public_key().unwrap(),
        &keys[2].raw_public_key().unwrap(),
    )
    .unwrap();
    assert_eq!(
        store
            .read("tenant_a", "research_01", "plan", 1, "p1")
            .unwrap()
            .evidence,
        evidence
    );
}

#[test]
fn cli_rejects_cross_tenant_wrong_signature_and_unbounded_stdin() {
    let dir = tempdir().unwrap();
    let engine_path = dir.path().join("om.db");
    let evidence_path = dir.path().join("proofs.db");
    let engine = OmEngine::open(&engine_path).unwrap();
    engine.register("tenant_a", contract()).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    engine
        .claim(
            "tenant_a",
            "research_01",
            Role::Planner,
            "plan",
            1,
            "p1",
            now,
        )
        .unwrap();
    let keys = [key(), key(), key()];
    let evidence = report(now);
    let body = envelope(&evidence, &keys[1]); // signed by the wrong authority
    assert!(!run(&evidence_path, &engine_path, &keys, &body, "tenant_a")
        .status
        .success());
    let good = envelope(&evidence, &keys[0]);
    assert!(!run(&evidence_path, &engine_path, &keys, &good, "tenant_b")
        .status
        .success());
    let too_large = vec![b'X'; 70_000];
    assert!(
        !run(&evidence_path, &engine_path, &keys, &too_large, "tenant_a")
            .status
            .success()
    );
    assert_eq!(
        engine.audit_head("tenant_a", "research_01").unwrap().events,
        1
    );
}

#[test]
fn cli_refuses_missing_or_invalid_trust_settings_before_opening_database() {
    let dir = tempdir().unwrap();
    let evidence_path = dir.path().join("proofs.db");
    let engine_path = dir.path().join("om.db");
    let result = Command::new(env!("CARGO_BIN_EXE_ocaf-architect-evidence"))
        .arg("--submit")
        .env("OCAF_EVIDENCE_DB", &evidence_path)
        .env("OCAF_EVIDENCE_OM_DB", &engine_path)
        .env("OCAF_EVIDENCE_TENANT", "tenant_a")
        .env_remove("OCAF_EVIDENCE_PLANNER_PUBKEY_ED25519")
        .env_remove("OCAF_EVIDENCE_ARCHITECT_PUBKEY_ED25519")
        .env_remove("OCAF_EVIDENCE_VERIFIER_PUBKEY_ED25519")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!evidence_path.exists());
    assert!(!engine_path.exists());
}

#[test]
fn cli_three_role_research_cycle_requires_distinct_attested_worker_signatures() {
    let dir = tempdir().unwrap();
    let engine_path = dir.path().join("om.db");
    let evidence_path = dir.path().join("proofs.db");
    let engine = OmEngine::open(&engine_path).unwrap();
    let mut workflow = contract();
    workflow.steps.extend([
        StepContract {
            id: "research".into(),
            role: Role::Architect,
            prerequisites: vec!["plan".into()],
            max_attempts: 2,
            max_cost_minor: 40,
            lease_seconds: 300,
        },
        StepContract {
            id: "verify".into(),
            role: Role::Verifier,
            prerequisites: vec!["research".into()],
            max_attempts: 1,
            max_cost_minor: 40,
            lease_seconds: 300,
        },
    ]);
    engine.register("tenant_a", workflow).unwrap();
    let keys = [key(), key(), key()];
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let store = ResearchEvidenceGate::open(
        &evidence_path,
        &keys[0].raw_public_key().unwrap(),
        &keys[1].raw_public_key().unwrap(),
        &keys[2].raw_public_key().unwrap(),
    )
    .unwrap();

    engine
        .claim(
            "tenant_a",
            "research_01",
            Role::Planner,
            "plan",
            1,
            "p1",
            now,
        )
        .unwrap();
    let plan = report(now);
    assert!(run(
        &evidence_path,
        &engine_path,
        &keys,
        &envelope(&plan, &keys[0]),
        "tenant_a"
    )
    .status
    .success());

    engine
        .claim(
            "tenant_a",
            "research_01",
            Role::Architect,
            "research",
            2,
            "r1",
            now,
        )
        .unwrap();
    let mut research = report(now);
    research.step = "research".into();
    research.role = Role::Architect;
    research.epoch = 2;
    research.attempt_id = "r1".into();
    research.sources = vec![
        SourceRecord {
            url: "https://example.com/benchmark-a".into(),
            retrieved_unix: now - 100,
            content_sha256: "a".repeat(64),
        },
        SourceRecord {
            url: "https://example.org/benchmark-b".into(),
            retrieved_unix: now - 100,
            content_sha256: "b".repeat(64),
        },
    ];
    let bad = run(
        &evidence_path,
        &engine_path,
        &keys,
        &envelope(&research, &keys[0]),
        "tenant_a",
    );
    assert!(!bad.status.success());
    assert!(run(
        &evidence_path,
        &engine_path,
        &keys,
        &envelope(&research, &keys[1]),
        "tenant_a"
    )
    .status
    .success());
    let research_sha = store
        .read("tenant_a", "research_01", "research", 2, "r1")
        .unwrap()
        .digest_sha256;

    engine
        .claim(
            "tenant_a",
            "research_01",
            Role::Verifier,
            "verify",
            3,
            "v1",
            now,
        )
        .unwrap();
    let mut review = report(now);
    review.step = "verify".into();
    review.role = Role::Verifier;
    review.epoch = 3;
    review.attempt_id = "v1".into();
    review.review_of_sha256 = Some("c".repeat(64));
    assert!(!run(
        &evidence_path,
        &engine_path,
        &keys,
        &envelope(&review, &keys[2]),
        "tenant_a"
    )
    .status
    .success());
    assert!(matches!(
        engine.status("tenant_a", "research_01", "verify"),
        Ok(StepStatus::Leased(_))
    ));
    review.review_of_sha256 = Some(research_sha);
    let good = run(
        &evidence_path,
        &engine_path,
        &keys,
        &envelope(&review, &keys[2]),
        "tenant_a",
    );
    assert!(
        good.status.success(),
        "{:?}",
        String::from_utf8_lossy(&good.stderr)
    );
    assert_eq!(
        engine.audit_head("tenant_a", "research_01").unwrap().events,
        6
    );
}
