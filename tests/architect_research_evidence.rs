use ocaf_runtime::{
    architect_research_evidence::{
        EvidenceError, ResearchEvidence, ResearchEvidenceGate, SourceRecord,
    },
    om_engine::OmEngine,
    omnidev_workflow::{Completion, Outcome, Role, StepContract, StepStatus, WorkflowContract},
};
use openssl::{
    pkey::{PKey, Private},
    sign::Signer,
};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

const NOW: u64 = 1_740_000_000;
fn private() -> PKey<Private> {
    PKey::generate_ed25519().unwrap()
}
fn sha(v: &[u8]) -> String {
    format!("{:x}", Sha256::digest(v))
}
fn contract() -> WorkflowContract {
    WorkflowContract {
        tenant: "tenant_a".into(),
        workflow_id: "research_period_1".into(),
        base_commit_sha: "a".repeat(40),
        deadline_unix: NOW + 1000,
        max_cost_minor: 300,
        steps: vec![
            StepContract {
                id: "plan".into(),
                role: Role::Planner,
                prerequisites: vec![],
                max_attempts: 1,
                max_cost_minor: 100,
                lease_seconds: 300,
            },
            StepContract {
                id: "research".into(),
                role: Role::Architect,
                prerequisites: vec!["plan".into()],
                max_attempts: 2,
                max_cost_minor: 100,
                lease_seconds: 300,
            },
            StepContract {
                id: "verify".into(),
                role: Role::Verifier,
                prerequisites: vec!["research".into()],
                max_attempts: 1,
                max_cost_minor: 100,
                lease_seconds: 300,
            },
        ],
    }
}
fn source(id: u8) -> SourceRecord {
    SourceRecord {
        url: format!("https://example.com/research/{id}"),
        retrieved_unix: NOW - 100,
        content_sha256: sha(&[id]),
    }
}
fn evidence(step: &str, role: Role, epoch: u64, attempt: &str) -> ResearchEvidence {
    ResearchEvidence {
        schema_version: 1,
        tenant: "tenant_a".into(),
        workflow_id: "research_period_1".into(),
        step: step.into(),
        role,
        epoch,
        attempt_id: attempt.into(),
        observed_at_unix: NOW + 5,
        actual_cost_minor: 5,
        outcome: Outcome::Passed,
        summary: "Evidence-backed finding; not a claim of independent source verification".into(),
        sources: if step == "research" {
            vec![source(1), source(2)]
        } else {
            vec![]
        },
        review_of_sha256: None,
    }
}
fn sign(key: &PKey<Private>, report: &ResearchEvidence) -> Vec<u8> {
    let bytes = report.signing_message().unwrap();
    let mut signer = Signer::new_without_digest(key).unwrap();
    signer.sign_oneshot_to_vec(&bytes).unwrap()
}
fn gate(path: &std::path::Path, keys: &[PKey<Private>; 3]) -> ResearchEvidenceGate {
    ResearchEvidenceGate::open(
        path,
        &keys[0].raw_public_key().unwrap(),
        &keys[1].raw_public_key().unwrap(),
        &keys[2].raw_public_key().unwrap(),
    )
    .unwrap()
}

#[test]
fn signed_three_role_evidence_completes_actual_om_engine_after_restart() {
    let dir = tempdir().unwrap();
    let evidence_path = dir.path().join("evidence.db");
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    engine.register("tenant_a", contract()).unwrap();
    let keys = [private(), private(), private()];
    let proof = gate(&evidence_path, &keys);
    let mut outputs: Vec<String> = Vec::new();
    for (step, role, idx, epoch, attempt) in [
        ("plan", Role::Planner, 0, 1, "plan_1"),
        ("research", Role::Architect, 1, 2, "research_1"),
        ("verify", Role::Verifier, 2, 3, "verify_1"),
    ] {
        engine
            .claim(
                "tenant_a",
                "research_period_1",
                role,
                step,
                epoch,
                attempt,
                NOW,
            )
            .unwrap();
        let mut report = evidence(step, role, epoch, attempt);
        report.review_of_sha256 = if step == "verify" {
            Some(outputs[1].clone())
        } else {
            None
        };
        let sig = sign(&keys[idx], &report);
        let result = proof
            .submit(&engine, "tenant_a", &report, &sig, NOW + 6)
            .unwrap();
        assert_eq!(result.completion, Completion::Recorded);
        assert_eq!(
            engine
                .status("tenant_a", "research_period_1", step)
                .unwrap(),
            StepStatus::Passed {
                evidence_sha256: result.digest_sha256.clone()
            }
        );
        let item = proof
            .read("tenant_a", "research_period_1", step, epoch, attempt)
            .unwrap();
        assert_eq!(item.digest_sha256, result.digest_sha256);
        assert_eq!(item.evidence, report);
        outputs.push(result.digest_sha256);
    }
    assert_eq!(
        engine
            .audit_head("tenant_a", "research_period_1")
            .unwrap()
            .events,
        6
    );
    drop(proof);
    let reopened = gate(&evidence_path, &keys);
    let persisted = reopened
        .read("tenant_a", "research_period_1", "verify", 3, "verify_1")
        .unwrap();
    assert_eq!(persisted.digest_sha256, outputs[2]);
    let mut replay = evidence("verify", Role::Verifier, 3, "verify_1");
    replay.review_of_sha256 = Some(outputs[1].clone());
    let receipt = reopened
        .submit(
            &engine,
            "tenant_a",
            &replay,
            &sign(&keys[2], &replay),
            NOW + 7,
        )
        .unwrap();
    assert_eq!(receipt.completion, Completion::Replay);
    assert_eq!(
        engine
            .audit_head("tenant_a", "research_period_1")
            .unwrap()
            .events,
        6
    );
}

#[test]
fn cannot_submit_unclaimed_work_wrong_signer_or_tenant() {
    let dir = tempdir().unwrap();
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    engine.register("tenant_a", contract()).unwrap();
    let keys = [private(), private(), private()];
    let gate = gate(&dir.path().join("evidence.db"), &keys);
    let plan = evidence("plan", Role::Planner, 1, "plan_1");
    assert!(matches!(
        gate.submit(&engine, "tenant_a", &plan, &sign(&keys[0], &plan), NOW + 6),
        Err(EvidenceError::LeaseNotActive)
    ));
    engine
        .claim(
            "tenant_a",
            "research_period_1",
            Role::Planner,
            "plan",
            1,
            "plan_1",
            NOW,
        )
        .unwrap();
    assert!(matches!(
        gate.submit(&engine, "tenant_b", &plan, &sign(&keys[0], &plan), NOW + 6),
        Err(EvidenceError::Denied)
    ));
    assert!(matches!(
        gate.submit(&engine, "tenant_a", &plan, &sign(&keys[1], &plan), NOW + 6),
        Err(EvidenceError::Signature)
    ));
    let mut forged = plan.clone();
    forged.actual_cost_minor = 99;
    assert!(matches!(
        gate.submit(
            &engine,
            "tenant_a",
            &forged,
            &sign(&keys[0], &plan),
            NOW + 6
        ),
        Err(EvidenceError::Signature)
    ));
}

#[test]
fn verifier_cannot_bypass_research_evidence_or_self_sign_review() {
    let dir = tempdir().unwrap();
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    engine.register("tenant_a", contract()).unwrap();
    let keys = [private(), private(), private()];
    let gate = gate(&dir.path().join("evidence.db"), &keys);
    let bad = evidence("verify", Role::Verifier, 1, "v");
    assert!(matches!(
        gate.submit(&engine, "tenant_a", &bad, &sign(&keys[2], &bad), NOW + 6),
        Err(EvidenceError::Invalid)
    ));
    let mut bad = evidence("verify", Role::Verifier, 1, "v");
    bad.review_of_sha256 = Some(sha(b"not-the-research"));
    assert!(matches!(
        gate.submit(&engine, "tenant_a", &bad, &sign(&keys[1], &bad), NOW + 6),
        Err(EvidenceError::Signature)
    ));
}

#[test]
fn rejects_unbounded_payload_missing_sources_and_bad_urls() {
    let dir = tempdir().unwrap();
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    engine.register("tenant_a", contract()).unwrap();
    let keys = [private(), private(), private()];
    let gate = gate(&dir.path().join("evidence.db"), &keys);
    let mut report = evidence("research", Role::Architect, 2, "r");
    report.sources.clear();
    assert!(matches!(
        gate.submit(
            &engine,
            "tenant_a",
            &report,
            &sign(&keys[1], &report),
            NOW + 6
        ),
        Err(EvidenceError::Invalid)
    ));
    report.sources = vec![source(1), source(2)];
    report.sources[0].url = "http://127.0.0.1/private".into();
    assert!(matches!(
        gate.submit(
            &engine,
            "tenant_a",
            &report,
            &sign(&keys[1], &report),
            NOW + 6
        ),
        Err(EvidenceError::Invalid)
    ));
    report.sources[0] = source(1);
    report.summary = "x".repeat(40_000);
    assert!(matches!(
        report.signing_message(),
        Err(EvidenceError::Invalid)
    ));
}

#[test]
fn tampering_with_signed_persisted_source_is_detected_on_read() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("evidence.db");
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    engine.register("tenant_a", contract()).unwrap();
    let keys = [private(), private(), private()];
    let gate = gate(&path, &keys);
    engine
        .claim(
            "tenant_a",
            "research_period_1",
            Role::Planner,
            "plan",
            1,
            "p",
            NOW,
        )
        .unwrap();
    let report = evidence("plan", Role::Planner, 1, "p");
    gate.submit(
        &engine,
        "tenant_a",
        &report,
        &sign(&keys[0], &report),
        NOW + 6,
    )
    .unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE architect_research_evidence SET evidence_json='{}' WHERE tenant='tenant_a'",
            [],
        )
        .unwrap();
    assert!(matches!(
        gate.read("tenant_a", "research_period_1", "plan", 1, "p"),
        Err(EvidenceError::Integrity)
    ));
}

#[test]
fn stale_epoch_is_fenced_without_state_mutation() {
    let dir = tempdir().unwrap();
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    engine.register("tenant_a", contract()).unwrap();
    let keys = [private(), private(), private()];
    let gate = gate(&dir.path().join("evidence.db"), &keys);
    engine
        .claim(
            "tenant_a",
            "research_period_1",
            Role::Planner,
            "plan",
            1,
            "p",
            NOW,
        )
        .unwrap();
    let report = evidence("plan", Role::Planner, 2, "p");
    assert!(matches!(
        gate.submit(
            &engine,
            "tenant_a",
            &report,
            &sign(&keys[0], &report),
            NOW + 6
        ),
        Err(EvidenceError::LeaseNotActive)
    ));
    assert_eq!(
        engine
            .audit_head("tenant_a", "research_period_1")
            .unwrap()
            .events,
        1
    );
}

#[test]
fn signed_failed_receipt_replay_is_idempotent_even_after_restart() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("evidence.db");
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    engine.register("tenant_a", contract()).unwrap();
    let keys = [private(), private(), private()];
    engine
        .claim(
            "tenant_a",
            "research_period_1",
            Role::Planner,
            "plan",
            1,
            "p",
            NOW,
        )
        .unwrap();
    let mut failed = evidence("plan", Role::Planner, 1, "p");
    failed.outcome = Outcome::Failed;
    let signature = sign(&keys[0], &failed);
    {
        let gate = gate(&path, &keys);
        let first = gate
            .submit(&engine, "tenant_a", &failed, &signature, NOW + 6)
            .unwrap();
        assert_eq!(first.completion, Completion::Recorded);
        assert!(matches!(
            engine.status("tenant_a", "research_period_1", "plan"),
            Ok(StepStatus::Halted)
        ));
    }
    let reopened = gate(&path, &keys);
    let replay = reopened
        .submit(&engine, "tenant_a", &failed, &signature, NOW + 500)
        .unwrap();
    assert_eq!(replay.completion, Completion::Replay);
    assert_eq!(
        engine
            .audit_head("tenant_a", "research_period_1")
            .unwrap()
            .events,
        2
    );
    let mut changed = failed.clone();
    changed.summary.push_str(" changed");
    assert!(matches!(
        reopened.submit(
            &engine,
            "tenant_a",
            &changed,
            &sign(&keys[0], &changed),
            NOW + 500
        ),
        Err(EvidenceError::LeaseNotActive)
    ));
}

#[test]
fn rejects_semantically_equal_but_noncanonical_database_json_with_rewritten_digest() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("evidence.db");
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    engine.register("tenant_a", contract()).unwrap();
    let keys = [private(), private(), private()];
    let gate = gate(&path, &keys);
    engine
        .claim(
            "tenant_a",
            "research_period_1",
            Role::Planner,
            "plan",
            1,
            "p",
            NOW,
        )
        .unwrap();
    let report = evidence("plan", Role::Planner, 1, "p");
    gate.submit(
        &engine,
        "tenant_a",
        &report,
        &sign(&keys[0], &report),
        NOW + 6,
    )
    .unwrap();

    let db = rusqlite::Connection::open(&path).unwrap();
    let original: String = db
        .query_row(
            "SELECT evidence_json FROM architect_research_evidence WHERE tenant='tenant_a'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let modified = original.replacen(r#","tenant":"#, r#", "tenant":"#, 1);
    assert_ne!(modified, original);
    let altered_digest = sha(modified.as_bytes());
    db.execute(
        "UPDATE architect_research_evidence SET evidence_json=?1,evidence_sha256=?2
         WHERE tenant='tenant_a'",
        rusqlite::params![modified, altered_digest],
    )
    .unwrap();
    assert!(matches!(
        gate.read("tenant_a", "research_period_1", "plan", 1, "p"),
        Err(EvidenceError::Integrity)
    ));
}
