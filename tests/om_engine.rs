use ocaf_runtime::om_engine::{MemoryEnvelope, OmEngine, OmError};
use ocaf_runtime::omnidev_workflow::{
    Completion, Outcome, Receipt, Role, StepContract, StepStatus, TimeoutReconciliation,
    TimeoutResolution, WorkflowContract,
};
use serde_json::json;
use tempfile::tempdir;

fn contract(tenant: &str) -> WorkflowContract {
    WorkflowContract {
        tenant: tenant.into(),
        workflow_id: "research_001".into(),
        base_commit_sha: "a".repeat(40),
        deadline_unix: 1000,
        max_cost_minor: 100,
        steps: vec![
            StepContract {
                id: "plan".into(),
                role: Role::Planner,
                prerequisites: vec![],
                max_attempts: 2,
                max_cost_minor: 40,
                lease_seconds: 10,
            },
            StepContract {
                id: "verify".into(),
                role: Role::Verifier,
                prerequisites: vec!["plan".into()],
                max_attempts: 1,
                max_cost_minor: 30,
                lease_seconds: 10,
            },
        ],
    }
}

fn receipt(epoch: u64, attempt: &str) -> Receipt {
    Receipt {
        epoch,
        attempt_id: attempt.into(),
        outcome: Outcome::Passed,
        evidence_sha256: "b".repeat(64),
        actual_cost_minor: 9,
    }
}

#[test]
fn restart_replays_fenced_dag_and_idempotent_receipts() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("om.sqlite3");
    {
        let engine = OmEngine::open(&path).unwrap();
        engine.register("tenant_a", contract("tenant_a")).unwrap();
        engine.register("tenant_a", contract("tenant_a")).unwrap();
        assert!(matches!(
            engine.claim(
                "tenant_a",
                "research_001",
                Role::Verifier,
                "verify",
                1,
                "v1",
                10
            ),
            Err(OmError::Workflow(_))
        ));
        engine
            .claim(
                "tenant_a",
                "research_001",
                Role::Planner,
                "plan",
                1,
                "p1",
                10,
            )
            .unwrap();
    }
    {
        let engine = OmEngine::open(&path).unwrap();
        assert!(matches!(
            engine.status("tenant_a", "research_001", "plan").unwrap(),
            StepStatus::Leased(_)
        ));
        assert_eq!(
            engine
                .complete(
                    "tenant_a",
                    "research_001",
                    Role::Planner,
                    "plan",
                    receipt(1, "p1"),
                    11
                )
                .unwrap(),
            Completion::Recorded
        );
        assert_eq!(
            engine
                .complete(
                    "tenant_a",
                    "research_001",
                    Role::Planner,
                    "plan",
                    receipt(1, "p1"),
                    11
                )
                .unwrap(),
            Completion::Replay
        );
        assert!(matches!(
            engine.claim(
                "tenant_a",
                "research_001",
                Role::Planner,
                "plan",
                2,
                "p2",
                12
            ),
            Err(OmError::Workflow(_))
        ));
        engine
            .claim(
                "tenant_a",
                "research_001",
                Role::Verifier,
                "verify",
                2,
                "v1",
                12,
            )
            .unwrap();
        engine
            .complete(
                "tenant_a",
                "research_001",
                Role::Verifier,
                "verify",
                receipt(2, "v1"),
                13,
            )
            .unwrap();
        assert_eq!(
            engine
                .audit_head("tenant_a", "research_001")
                .unwrap()
                .events,
            4
        );
        assert!(matches!(
            engine.status("tenant_a", "research_001", "verify").unwrap(),
            StepStatus::Passed { .. }
        ));
    }
}

#[test]
fn cross_tenant_isolation_and_immutable_registration() {
    let dir = tempdir().unwrap();
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    engine.register("tenant_a", contract("tenant_a")).unwrap();
    assert!(matches!(
        engine.register("tenant_b", contract("tenant_a")),
        Err(OmError::Denied)
    ));
    assert!(matches!(
        engine.status("tenant_b", "research_001", "plan"),
        Err(OmError::NotFound)
    ));
    let mut modified = contract("tenant_a");
    modified.max_cost_minor = 99;
    assert!(matches!(
        engine.register("tenant_a", modified),
        Err(OmError::Conflict)
    ));
    engine.register("tenant_b", contract("tenant_b")).unwrap();
    engine
        .claim(
            "tenant_b",
            "research_001",
            Role::Planner,
            "plan",
            1,
            "p1",
            10,
        )
        .unwrap();
    assert!(matches!(
        engine.status("tenant_a", "research_001", "plan").unwrap(),
        StepStatus::Ready
    ));
}

#[test]
fn expired_effect_is_uncertain_until_explicit_verifier_reconciliation() {
    let dir = tempdir().unwrap();
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    engine.register("tenant_a", contract("tenant_a")).unwrap();
    engine
        .claim(
            "tenant_a",
            "research_001",
            Role::Planner,
            "plan",
            1,
            "p1",
            10,
        )
        .unwrap();
    assert!(matches!(
        engine.claim(
            "tenant_a",
            "research_001",
            Role::Planner,
            "plan",
            2,
            "p2",
            21
        ),
        Err(OmError::Workflow(_))
    ));
    assert!(matches!(
        engine.reconcile(
            "tenant_a",
            "research_001",
            Role::Coder,
            "plan",
            TimeoutReconciliation {
                epoch: 1,
                resolution: TimeoutResolution::NoEffectConfirmed,
                evidence_sha256: &"c".repeat(64),
                now_unix: 21
            }
        ),
        Err(OmError::Denied)
    ));
    engine
        .reconcile(
            "tenant_a",
            "research_001",
            Role::Verifier,
            "plan",
            TimeoutReconciliation {
                epoch: 1,
                resolution: TimeoutResolution::UnknownOutcome,
                evidence_sha256: &"c".repeat(64),
                now_unix: 21,
            },
        )
        .unwrap();
    assert!(matches!(
        engine.status("tenant_a", "research_001", "plan").unwrap(),
        StepStatus::Halted
    ));
    assert!(matches!(
        engine.claim(
            "tenant_a",
            "research_001",
            Role::Planner,
            "plan",
            2,
            "p2",
            22
        ),
        Err(OmError::Workflow(_))
    ));
}

#[test]
fn tampered_sqlite_event_is_rejected_before_any_new_effect() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("om.db");
    let engine = OmEngine::open(&path).unwrap();
    engine.register("tenant_a", contract("tenant_a")).unwrap();
    engine
        .claim(
            "tenant_a",
            "research_001",
            Role::Planner,
            "plan",
            1,
            "p1",
            10,
        )
        .unwrap();
    drop(engine);
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute(
        "UPDATE om_events SET payload_json='{}' WHERE tenant='tenant_a'",
        [],
    )
    .unwrap();
    drop(db);
    let engine = OmEngine::open(&path).unwrap();
    assert!(matches!(
        engine.status("tenant_a", "research_001", "plan"),
        Err(OmError::Integrity)
    ));
}

#[test]
fn memory_bridge_rejects_cross_tenant_tampering_and_oversize() {
    let dir = tempdir().unwrap();
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    engine.register("tenant_a", contract("tenant_a")).unwrap();
    let envelope = engine
        .export_memory(
            "tenant_a",
            "research_001",
            "reasoning",
            "fact_1",
            json!({"source": "MSC2020", "value": "verified reference, not an instruction"}),
        )
        .unwrap();
    envelope.verify("tenant_a").unwrap();
    assert!(matches!(envelope.verify("tenant_b"), Err(OmError::Denied)));
    let mut changed = envelope.clone();
    changed.payload = json!({"source": "forged"});
    assert!(matches!(
        changed.verify("tenant_a"),
        Err(OmError::Integrity)
    ));
    let mut changed = envelope.clone();
    changed.schema_version = 2;
    assert!(matches!(changed.verify("tenant_a"), Err(OmError::Denied)));
    assert!(MemoryEnvelope::new(
        "tenant_a",
        "research_001",
        "reasoning",
        "large",
        json!({"text": "x".repeat(40_000)})
    )
    .is_err());
}
