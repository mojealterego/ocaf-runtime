use ocaf_runtime::{
    architect_research_evidence::{ResearchEvidence, ResearchEvidenceGate, SourceRecord},
    architect_research_finalizer::{finalize_due, FinalizeError},
    architect_research_schedule::{ArchitectResearchScheduler, ResearchKind, RunState},
    architect_workflow_bridge::register_due,
    om_engine::OmEngine,
    omnidev_workflow::{Outcome, Role, StepStatus},
};
use openssl::{
    pkey::{PKey, Private},
    sign::Signer,
};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

const NOW: i64 = 1_740_000_000;
fn digest(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
fn keys() -> [PKey<Private>; 3] {
    [
        PKey::generate_ed25519().unwrap(),
        PKey::generate_ed25519().unwrap(),
        PKey::generate_ed25519().unwrap(),
    ]
}
fn gate(db: &std::path::Path, k: &[PKey<Private>; 3]) -> ResearchEvidenceGate {
    ResearchEvidenceGate::open(
        db,
        &k[0].raw_public_key().unwrap(),
        &k[1].raw_public_key().unwrap(),
        &k[2].raw_public_key().unwrap(),
    )
    .unwrap()
}
fn source(i: u8) -> SourceRecord {
    SourceRecord {
        url: format!("https://example.com/source/{i}"),
        retrieved_unix: (NOW - 8) as u64,
        content_sha256: digest(&[i]),
    }
}
fn signed(k: &PKey<Private>, report: &ResearchEvidence) -> Vec<u8> {
    Signer::new_without_digest(k)
        .unwrap()
        .sign_oneshot_to_vec(&report.signing_message().unwrap())
        .unwrap()
}
#[allow(clippy::too_many_arguments)]
fn complete_role(
    engine: &OmEngine,
    evidence_store: &ResearchEvidenceGate,
    tenant: &str,
    id: &str,
    key: &PKey<Private>,
    step: &str,
    role: Role,
    epoch: u64,
    review: Option<String>,
    outcome: Outcome,
) -> String {
    let attempt = format!("{step}_{epoch}");
    engine
        .claim(tenant, id, role, step, epoch, &attempt, NOW as u64 + epoch)
        .unwrap();
    let report = ResearchEvidence {
        schema_version: 1,
        tenant: tenant.into(),
        workflow_id: id.into(),
        step: step.into(),
        role,
        epoch,
        attempt_id: attempt,
        observed_at_unix: (NOW + epoch as i64 + 1) as u64,
        actual_cost_minor: 5,
        outcome,
        summary: "Signed fixture for deterministic OM workflow contract integration".into(),
        sources: if step == "research" {
            vec![source(1), source(2)]
        } else {
            vec![]
        },
        review_of_sha256: review,
    };
    evidence_store
        .submit(
            engine,
            tenant,
            &report,
            &signed(key, &report),
            (NOW + epoch as i64 + 2) as u64,
        )
        .unwrap()
        .digest_sha256
}

#[test]
fn verified_three_role_evidence_seals_scheduler_and_survives_restart() {
    let dir = tempdir().unwrap();
    let schedule_path = dir.path().join("schedule.sqlite");
    let engine_path = dir.path().join("om.sqlite");
    let evidence_path = dir.path().join("evidence.sqlite");
    let scheduler = ArchitectResearchScheduler::open(&schedule_path).unwrap();
    let engine = OmEngine::open(&engine_path).unwrap();
    scheduler.tick("tenant_a", "fleet_a", NOW).unwrap();
    let rec = register_due(
        &scheduler,
        &engine,
        "tenant_a",
        "fleet_a",
        &"a".repeat(40),
        90,
        NOW,
        64,
    )
    .unwrap()
    .into_iter()
    .find(|x| x.kind == ResearchKind::Competitive)
    .unwrap();
    let k = keys();
    let evidence = gate(&evidence_path, &k);
    assert!(matches!(
        finalize_due(
            &scheduler,
            &engine,
            &evidence,
            "tenant_a",
            "fleet_a",
            &rec,
            NOW + 1
        ),
        Err(FinalizeError::Unverified)
    ));
    let _plan = complete_role(
        &engine,
        &evidence,
        "tenant_a",
        &rec.workflow_id,
        &k[0],
        "plan",
        Role::Planner,
        1,
        None,
        Outcome::Passed,
    );
    assert!(matches!(
        finalize_due(
            &scheduler,
            &engine,
            &evidence,
            "tenant_a",
            "fleet_a",
            &rec,
            NOW + 5
        ),
        Err(FinalizeError::Unverified)
    ));
    let research = complete_role(
        &engine,
        &evidence,
        "tenant_a",
        &rec.workflow_id,
        &k[1],
        "research",
        Role::Architect,
        2,
        None,
        Outcome::Passed,
    );
    let verified = complete_role(
        &engine,
        &evidence,
        "tenant_a",
        &rec.workflow_id,
        &k[2],
        "verify",
        Role::Verifier,
        3,
        Some(research),
        Outcome::Passed,
    );
    assert_eq!(
        engine
            .audit_head("tenant_a", &rec.workflow_id)
            .unwrap()
            .events,
        6
    );
    let first = finalize_due(
        &scheduler,
        &engine,
        &evidence,
        "tenant_a",
        "fleet_a",
        &rec,
        NOW + 20,
    )
    .unwrap();
    assert_eq!(first.evidence_sha256, verified);
    assert!(!first.replay);
    let repeat = finalize_due(
        &scheduler,
        &engine,
        &evidence,
        "tenant_a",
        "fleet_a",
        &rec,
        NOW + 21,
    )
    .unwrap();
    assert_eq!(repeat.evidence_sha256, verified);
    assert!(repeat.replay);
    let current = scheduler
        .snapshot("tenant_a", "fleet_a", rec.kind, rec.period_start_unix)
        .unwrap();
    assert_eq!(current.state, RunState::Succeeded);
    assert_eq!(
        current.last_evidence_sha256.as_deref(),
        Some(verified.as_str())
    );
    let remaining = scheduler
        .list_due("tenant_a", "fleet_a", NOW + 21, 64)
        .unwrap();
    assert_eq!(remaining.len(), 2);
    drop(evidence);
    drop(engine);
    drop(scheduler);
    let scheduler = ArchitectResearchScheduler::open(&schedule_path).unwrap();
    let engine = OmEngine::open(&engine_path).unwrap();
    let evidence = gate(&evidence_path, &k);
    assert!(
        finalize_due(
            &scheduler,
            &engine,
            &evidence,
            "tenant_a",
            "fleet_a",
            &rec,
            NOW + 30
        )
        .unwrap()
        .replay
    );
}

#[test]
fn om_passed_without_signed_evidence_must_not_seal_queue() {
    let dir = tempdir().unwrap();
    let scheduler = ArchitectResearchScheduler::open(dir.path().join("queue.db")).unwrap();
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    let k = keys();
    let evidence = gate(&dir.path().join("evidence.db"), &k);
    scheduler.tick("tenant", "fleet", NOW).unwrap();
    let rec = register_due(
        &scheduler,
        &engine,
        "tenant",
        "fleet",
        &"b".repeat(40),
        90,
        NOW,
        1,
    )
    .unwrap()
    .remove(0);
    for (step, role, epoch) in [
        ("plan", Role::Planner, 1),
        ("research", Role::Architect, 2),
        ("verify", Role::Verifier, 3),
    ] {
        let attempt = format!("{step}_{epoch}");
        engine
            .claim(
                "tenant",
                &rec.workflow_id,
                role,
                step,
                epoch,
                &attempt,
                NOW as u64 + epoch,
            )
            .unwrap();
        engine
            .complete(
                "tenant",
                &rec.workflow_id,
                role,
                step,
                ocaf_runtime::omnidev_workflow::Receipt {
                    epoch,
                    attempt_id: attempt,
                    outcome: Outcome::Passed,
                    evidence_sha256: digest(step.as_bytes()),
                    actual_cost_minor: 5,
                },
                NOW as u64 + epoch + 1,
            )
            .unwrap();
    }
    assert!(matches!(
        engine.status("tenant", &rec.workflow_id, "verify"),
        Ok(StepStatus::Passed { .. })
    ));
    assert!(matches!(
        finalize_due(
            &scheduler,
            &engine,
            &evidence,
            "tenant",
            "fleet",
            &rec,
            NOW + 20
        ),
        Err(FinalizeError::Evidence(_))
    ));
    assert_eq!(
        scheduler
            .snapshot("tenant", "fleet", rec.kind, rec.period_start_unix)
            .unwrap()
            .state,
        RunState::Ready
    );
}

#[test]
fn wrong_fleet_tenant_or_lease_cannot_finalize() {
    let dir = tempdir().unwrap();
    let scheduler = ArchitectResearchScheduler::open(dir.path().join("queue.db")).unwrap();
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    let k = keys();
    let evidence = gate(&dir.path().join("evidence.db"), &k);
    scheduler.tick("tenant", "fleet", NOW).unwrap();
    let rec = register_due(
        &scheduler,
        &engine,
        "tenant",
        "fleet",
        &"b".repeat(40),
        90,
        NOW,
        1,
    )
    .unwrap()
    .remove(0);
    assert!(matches!(
        finalize_due(
            &scheduler,
            &engine,
            &evidence,
            "tenant",
            "other_fleet",
            &rec,
            NOW
        ),
        Err(FinalizeError::Mismatch)
    ));
    assert!(finalize_due(
        &scheduler,
        &engine,
        &evidence,
        "other_tenant",
        "fleet",
        &rec,
        NOW
    )
    .is_err());
    let lease = scheduler
        .claim(
            "tenant",
            "fleet",
            rec.kind,
            rec.period_start_unix,
            "worker",
            NOW,
            30,
        )
        .unwrap();
    assert!(matches!(
        finalize_due(
            &scheduler,
            &engine,
            &evidence,
            "tenant",
            "fleet",
            &rec,
            NOW + 1
        ),
        Err(FinalizeError::Unverified)
    ));
    assert_eq!(
        scheduler
            .snapshot("tenant", "fleet", rec.kind, rec.period_start_unix)
            .unwrap()
            .epoch,
        lease.epoch
    );
}
