use ocaf_runtime::{
    architect_research_schedule::{ArchitectResearchScheduler, ResearchKind},
    architect_workflow_bridge::{register_due, BridgeError},
    om_engine::{OmEngine, OmError},
    omnidev_workflow::{Completion, Outcome, Receipt, Role, StepStatus, WorkflowError},
};
use tempfile::tempdir;

const NOW: i64 = 1_740_000_000;
fn baseline() -> String {
    "c".repeat(40)
}

#[test]
fn due_research_jobs_are_registered_in_real_om_engine_and_replayed() {
    let dir = tempdir().unwrap();
    let scheduler = ArchitectResearchScheduler::open(dir.path().join("schedule.db")).unwrap();
    let engine = OmEngine::open(dir.path().join("engine.db")).unwrap();
    scheduler.tick("tenant", "fleet", NOW).unwrap();
    let workflows = register_due(
        &scheduler,
        &engine,
        "tenant",
        "fleet",
        &baseline(),
        90,
        NOW,
        64,
    )
    .unwrap();
    assert_eq!(workflows.len(), 3);
    assert_eq!(
        workflows
            .iter()
            .filter(|r| r.kind == ResearchKind::Competitive)
            .count(),
        1
    );
    for record in &workflows {
        assert!(matches!(
            engine.status("tenant", &record.workflow_id, "plan"),
            Ok(StepStatus::Ready)
        ));
        assert!(matches!(
            engine.status("tenant", &record.workflow_id, "research"),
            Ok(StepStatus::Ready)
        ));
        assert!(matches!(
            engine.status("tenant", &record.workflow_id, "verify"),
            Ok(StepStatus::Ready)
        ));
        assert_eq!(
            engine
                .audit_head("tenant", &record.workflow_id)
                .unwrap()
                .events,
            0
        );
        assert!(matches!(
            engine.claim(
                "tenant",
                &record.workflow_id,
                Role::Architect,
                "research",
                1,
                "a1",
                NOW as u64
            ),
            Err(OmError::Workflow(WorkflowError::NotReady))
        ));
    }
    assert_eq!(
        register_due(
            &scheduler,
            &engine,
            "tenant",
            "fleet",
            &baseline(),
            90,
            NOW,
            64
        )
        .unwrap(),
        workflows
    );
}

#[test]
fn actual_om_workflow_restricts_roles_preserves_fencing_and_receipts() {
    let dir = tempdir().unwrap();
    let scheduler = ArchitectResearchScheduler::open(dir.path().join("q.db")).unwrap();
    let engine = OmEngine::open(dir.path().join("engine.db")).unwrap();
    scheduler.tick("a", "fleet", NOW).unwrap();
    let record = register_due(&scheduler, &engine, "a", "fleet", &baseline(), 90, NOW, 1)
        .unwrap()
        .remove(0);
    assert!(matches!(
        engine.claim(
            "a",
            &record.workflow_id,
            Role::Coder,
            "plan",
            1,
            "bad",
            NOW as u64
        ),
        Err(OmError::Workflow(WorkflowError::Denied))
    ));
    engine
        .claim(
            "a",
            &record.workflow_id,
            Role::Planner,
            "plan",
            1,
            "p1",
            NOW as u64,
        )
        .unwrap();
    let completion = engine
        .complete(
            "a",
            &record.workflow_id,
            Role::Planner,
            "plan",
            Receipt {
                epoch: 1,
                attempt_id: "p1".into(),
                outcome: Outcome::Passed,
                evidence_sha256: "d".repeat(64),
                actual_cost_minor: 2,
            },
            (NOW + 1) as u64,
        )
        .unwrap();
    assert_eq!(completion, Completion::Recorded);
    engine
        .claim(
            "a",
            &record.workflow_id,
            Role::Architect,
            "research",
            2,
            "r1",
            (NOW + 2) as u64,
        )
        .unwrap();
    assert!(matches!(
        engine.status("b", &record.workflow_id, "plan"),
        Err(OmError::NotFound)
    ));
    assert_eq!(
        engine.audit_head("a", &record.workflow_id).unwrap().events,
        3
    );
}

#[test]
fn conflicting_base_revision_or_policy_never_overwrites_registered_workflow() {
    let dir = tempdir().unwrap();
    let scheduler = ArchitectResearchScheduler::open(dir.path().join("q.db")).unwrap();
    let path = dir.path().join("om.db");
    scheduler.tick("tenant", "fleet", NOW).unwrap();
    {
        let engine = OmEngine::open(&path).unwrap();
        register_due(
            &scheduler,
            &engine,
            "tenant",
            "fleet",
            &baseline(),
            90,
            NOW,
            64,
        )
        .unwrap();
    }
    let engine = OmEngine::open(&path).unwrap();
    assert!(matches!(
        register_due(
            &scheduler,
            &engine,
            "tenant",
            "fleet",
            &"a".repeat(40),
            90,
            NOW,
            64
        ),
        Err(BridgeError::Engine(OmError::Conflict))
    ));
    assert!(matches!(
        register_due(
            &scheduler,
            &engine,
            "tenant",
            "fleet",
            &baseline(),
            91,
            NOW,
            64
        ),
        Err(BridgeError::Engine(OmError::Conflict))
    ));
    assert_eq!(
        register_due(
            &scheduler,
            &engine,
            "tenant",
            "fleet",
            &baseline(),
            90,
            NOW,
            64
        )
        .unwrap()
        .len(),
        3
    );
}

#[test]
fn tenant_and_fleet_bound_workflow_ids_cannot_cross_access() {
    let dir = tempdir().unwrap();
    let scheduler = ArchitectResearchScheduler::open(dir.path().join("q.db")).unwrap();
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    scheduler.tick("alice", "fleet-1", NOW).unwrap();
    scheduler.tick("bob", "fleet-1", NOW).unwrap();
    scheduler.tick("alice", "fleet-2", NOW).unwrap();
    let a = register_due(
        &scheduler,
        &engine,
        "alice",
        "fleet-1",
        &baseline(),
        90,
        NOW,
        1,
    )
    .unwrap()
    .remove(0);
    let b = register_due(
        &scheduler,
        &engine,
        "alice",
        "fleet-2",
        &baseline(),
        90,
        NOW,
        1,
    )
    .unwrap()
    .remove(0);
    assert_ne!(a.workflow_id, b.workflow_id);
    assert!(matches!(
        engine.status("bob", &a.workflow_id, "plan"),
        Err(OmError::NotFound)
    ));
    let c = register_due(
        &scheduler,
        &engine,
        "bob",
        "fleet-1",
        &baseline(),
        90,
        NOW,
        1,
    )
    .unwrap()
    .remove(0);
    assert_eq!(c.workflow_id, a.workflow_id);
    assert!(matches!(
        engine.status("bob", &c.workflow_id, "plan"),
        Ok(StepStatus::Ready)
    ));
}

#[test]
fn malformed_policy_or_sha_is_rejected_before_registration() {
    let dir = tempdir().unwrap();
    let scheduler = ArchitectResearchScheduler::open(dir.path().join("q.db")).unwrap();
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    scheduler.tick("tenant", "fleet", NOW).unwrap();
    for budget in [0, 1, 2, 1_000_001] {
        assert!(matches!(
            register_due(
                &scheduler,
                &engine,
                "tenant",
                "fleet",
                &baseline(),
                budget,
                NOW,
                1
            ),
            Err(BridgeError::Invalid)
        ));
    }
    assert!(matches!(
        register_due(
            &scheduler,
            &engine,
            "tenant",
            "fleet",
            "not-a-commit",
            90,
            NOW,
            1
        ),
        Err(BridgeError::Invalid)
    ));
    assert!(matches!(
        register_due(
            &scheduler,
            &engine,
            "tenant",
            "fleet",
            &baseline(),
            90,
            NOW,
            0
        ),
        Err(BridgeError::Invalid)
    ));
    assert!(matches!(
        register_due(
            &scheduler,
            &engine,
            "tenant",
            "fleet",
            &baseline(),
            90,
            NOW,
            65
        ),
        Err(BridgeError::Invalid)
    ));
}
