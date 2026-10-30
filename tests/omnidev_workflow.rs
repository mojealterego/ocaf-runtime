use ocaf_runtime::omnidev_workflow::{
    Completion, OmniDevWorkflow, Outcome, Receipt, Role, StepContract, StepStatus,
    TimeoutResolution, WorkflowContract, WorkflowError,
};

fn digest() -> String {
    "a".repeat(64)
}
fn spec() -> WorkflowContract {
    WorkflowContract {
        tenant: "tenantA".into(),
        workflow_id: "candidate01".into(),
        base_commit_sha: "b".repeat(40),
        deadline_unix: 1_000,
        max_cost_minor: 50,
        steps: vec![
            StepContract {
                id: "architect".into(),
                role: Role::Architect,
                prerequisites: vec![],
                max_attempts: 2,
                max_cost_minor: 30,
                lease_seconds: 10,
            },
            StepContract {
                id: "critic".into(),
                role: Role::SecurityCritic,
                prerequisites: vec!["architect".into()],
                max_attempts: 2,
                max_cost_minor: 20,
                lease_seconds: 10,
            },
        ],
    }
}
fn receipt(epoch: u64, attempt: &str, outcome: Outcome, cost: u64) -> Receipt {
    Receipt {
        epoch,
        attempt_id: attempt.into(),
        outcome,
        evidence_sha256: digest(),
        actual_cost_minor: cost,
    }
}

#[test]
fn valid_dependency_chain_needs_independent_roles() {
    let mut w = OmniDevWorkflow::new(spec()).unwrap();
    assert_eq!(
        w.claim("tenantA", Role::SecurityCritic, "critic", 1, "r1", 10),
        Err(WorkflowError::NotReady)
    );
    let claim = w
        .claim("tenantA", Role::Architect, "architect", 1, "r1", 10)
        .unwrap();
    assert_eq!(claim.expires_at, 20);
    assert_eq!(
        w.complete(
            "tenantA",
            Role::Architect,
            "architect",
            receipt(1, "r1", Outcome::Passed, 10),
            11
        ),
        Ok(Completion::Recorded)
    );
    assert_eq!(w.budget_state(), (10, 0));
    w.claim("tenantA", Role::SecurityCritic, "critic", 2, "r2", 12)
        .unwrap();
    w.complete(
        "tenantA",
        Role::SecurityCritic,
        "critic",
        receipt(2, "r2", Outcome::Passed, 20),
        13,
    )
    .unwrap();
    assert!(w.is_completed());
    assert_eq!(w.audit_head().0, 4);
}

#[test]
fn tenant_and_role_enforcement_is_fail_closed() {
    let mut w = OmniDevWorkflow::new(spec()).unwrap();
    assert_eq!(
        w.claim("tenantB", Role::Architect, "architect", 1, "x", 10),
        Err(WorkflowError::Denied)
    );
    assert_eq!(
        w.claim("tenantA", Role::Coder, "architect", 1, "x", 10),
        Err(WorkflowError::Denied)
    );
    assert_eq!(w.status("tenantB", "architect"), Err(WorkflowError::Denied));
    w.claim("tenantA", Role::Architect, "architect", 1, "x", 10)
        .unwrap();
    assert_eq!(
        w.complete(
            "tenantB",
            Role::Architect,
            "architect",
            receipt(1, "x", Outcome::Passed, 1),
            11
        ),
        Err(WorkflowError::Denied)
    );
    assert_eq!(
        w.complete(
            "tenantA",
            Role::Coder,
            "architect",
            receipt(1, "x", Outcome::Passed, 1),
            11
        ),
        Err(WorkflowError::Denied)
    );
}

#[test]
fn replay_receipt_is_idempotent_but_conflict_is_denied() {
    let mut w = OmniDevWorkflow::new(spec()).unwrap();
    w.claim("tenantA", Role::Architect, "architect", 1, "x", 10)
        .unwrap();
    let r = receipt(1, "x", Outcome::Passed, 10);
    assert_eq!(
        w.complete("tenantA", Role::Architect, "architect", r.clone(), 11),
        Ok(Completion::Recorded)
    );
    assert_eq!(
        w.complete("tenantA", Role::Architect, "architect", r, 12),
        Ok(Completion::Replay)
    );
    assert_eq!(w.budget_state(), (10, 0));
    assert_eq!(
        w.complete(
            "tenantA",
            Role::Architect,
            "architect",
            receipt(1, "x", Outcome::Passed, 20),
            13
        ),
        Err(WorkflowError::ReceiptConflict)
    );
}

#[test]
fn stale_epochs_and_wrong_attempts_cannot_finalize() {
    let mut w = OmniDevWorkflow::new(spec()).unwrap();
    w.claim("tenantA", Role::Architect, "architect", 5, "x", 10)
        .unwrap();
    assert_eq!(
        w.claim("tenantA", Role::Architect, "architect", 5, "y", 11),
        Err(WorkflowError::Fenced)
    );
    assert_eq!(
        w.complete(
            "tenantA",
            Role::Architect,
            "architect",
            receipt(4, "x", Outcome::Passed, 10),
            11
        ),
        Err(WorkflowError::Fenced)
    );
    assert_eq!(
        w.complete(
            "tenantA",
            Role::Architect,
            "architect",
            receipt(5, "y", Outcome::Passed, 10),
            11
        ),
        Err(WorkflowError::Fenced)
    );
}

#[test]
fn expired_effect_never_automatically_retries() {
    let mut w = OmniDevWorkflow::new(spec()).unwrap();
    w.claim("tenantA", Role::Architect, "architect", 1, "x", 10)
        .unwrap();
    assert_eq!(
        w.claim("tenantA", Role::Architect, "architect", 2, "y", 20),
        Err(WorkflowError::Unresolved)
    );
    assert_eq!(
        w.complete(
            "tenantA",
            Role::Architect,
            "architect",
            receipt(1, "x", Outcome::Passed, 1),
            20
        ),
        Err(WorkflowError::Unresolved)
    );
    assert_eq!(
        w.reconcile_timeout(
            "tenantA",
            Role::Coder,
            "architect",
            1,
            TimeoutResolution::NoEffectConfirmed,
            &digest(),
            20
        ),
        Err(WorkflowError::Denied)
    );
    w.reconcile_timeout(
        "tenantA",
        Role::Verifier,
        "architect",
        1,
        TimeoutResolution::NoEffectConfirmed,
        &digest(),
        20,
    )
    .unwrap();
    w.claim("tenantA", Role::Architect, "architect", 2, "y", 21)
        .unwrap();
    assert_eq!(
        w.complete(
            "tenantA",
            Role::Architect,
            "architect",
            receipt(1, "x", Outcome::Passed, 1),
            22
        ),
        Err(WorkflowError::Fenced)
    );
}

#[test]
fn unresolved_timeout_halts_and_is_not_readmitted() {
    let mut w = OmniDevWorkflow::new(spec()).unwrap();
    w.claim("tenantA", Role::Architect, "architect", 1, "x", 10)
        .unwrap();
    w.reconcile_timeout(
        "tenantA",
        Role::Verifier,
        "architect",
        1,
        TimeoutResolution::UnknownOutcome,
        &digest(),
        20,
    )
    .unwrap();
    assert!(matches!(
        w.status("tenantA", "architect"),
        Ok(StepStatus::Halted)
    ));
    assert_eq!(
        w.claim("tenantA", Role::Architect, "architect", 2, "y", 21),
        Err(WorkflowError::BusyOrTerminal)
    );
}

#[test]
fn failed_attempt_is_bounded_and_cost_reservations_are_enforced() {
    let mut w = OmniDevWorkflow::new(spec()).unwrap();
    w.claim("tenantA", Role::Architect, "architect", 1, "x", 10)
        .unwrap();
    assert_eq!(w.budget_state(), (0, 30));
    assert_eq!(
        w.claim("tenantA", Role::Architect, "architect", 2, "y", 11),
        Err(WorkflowError::BusyOrTerminal)
    );
    w.complete(
        "tenantA",
        Role::Architect,
        "architect",
        receipt(1, "x", Outcome::Failed, 10),
        11,
    )
    .unwrap();
    w.claim("tenantA", Role::Architect, "architect", 2, "y", 12)
        .unwrap();
    w.complete(
        "tenantA",
        Role::Architect,
        "architect",
        receipt(2, "y", Outcome::Failed, 10),
        13,
    )
    .unwrap();
    assert_eq!(
        w.claim("tenantA", Role::Architect, "architect", 3, "z", 14),
        Err(WorkflowError::BusyOrTerminal)
    );
}

#[test]
fn budget_does_not_double_reserve_parallel_jobs() {
    let mut s = spec();
    s.steps[1].prerequisites.clear();
    let mut w = OmniDevWorkflow::new(s).unwrap();
    w.claim("tenantA", Role::Architect, "architect", 1, "x", 10)
        .unwrap();
    assert_eq!(w.budget_state(), (0, 30));
    w.claim("tenantA", Role::SecurityCritic, "critic", 2, "y", 11)
        .unwrap();
    assert_eq!(w.budget_state(), (0, 50));
    assert_eq!(
        w.complete(
            "tenantA",
            Role::Architect,
            "architect",
            receipt(1, "x", Outcome::Passed, 31),
            12
        ),
        Err(WorkflowError::BudgetOrDeadline)
    );
}

#[test]
fn invalid_graphs_and_contracts_are_rejected() {
    let mut s = spec();
    s.steps[0].prerequisites.push("critic".into());
    assert!(matches!(
        OmniDevWorkflow::new(s),
        Err(WorkflowError::DependencyGraph)
    ));
    let mut s = spec();
    s.steps[0].prerequisites.push("missing".into());
    assert!(matches!(
        OmniDevWorkflow::new(s),
        Err(WorkflowError::DependencyGraph)
    ));
    let mut s = spec();
    s.steps[0].max_attempts = 0;
    assert!(matches!(
        OmniDevWorkflow::new(s),
        Err(WorkflowError::Invalid)
    ));
    let mut s = spec();
    s.tenant = "../bad".into();
    assert!(matches!(
        OmniDevWorkflow::new(s),
        Err(WorkflowError::Invalid)
    ));
}

#[test]
fn deadline_and_lease_expiration_are_enforced() {
    let mut w = OmniDevWorkflow::new(spec()).unwrap();
    assert_eq!(
        w.claim("tenantA", Role::Architect, "architect", 1, "x", 1_000),
        Err(WorkflowError::BudgetOrDeadline)
    );
    w.claim("tenantA", Role::Architect, "architect", 1, "x", 995)
        .unwrap();
    assert_eq!(
        w.complete(
            "tenantA",
            Role::Architect,
            "architect",
            receipt(1, "x", Outcome::Passed, 0),
            1_000
        ),
        Err(WorkflowError::Unresolved)
    );
}
