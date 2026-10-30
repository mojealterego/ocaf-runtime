use ocaf_runtime::finops_ledger::{FinOpsError, FinOpsLedger, ReservationState};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn reserves_exact_budget_and_denies_overspend() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = FinOpsLedger::open(dir.path().join("finops.sqlite")).unwrap();
    assert!(matches!(
        ledger.reserve("tenant", "agent", "r1", 1),
        Err(FinOpsError::BudgetNotConfigured)
    ));
    ledger.configure_budget("tenant", "agent", 100).unwrap();
    assert_eq!(
        ledger.reserve("tenant", "agent", "r1", 70).unwrap(),
        ReservationState::Reserved
    );
    assert!(matches!(
        ledger.reserve("tenant", "agent", "r2", 31),
        Err(FinOpsError::BudgetExceeded)
    ));
    assert_eq!(
        ledger.reserve("tenant", "agent", "r2", 30).unwrap(),
        ReservationState::Reserved
    );
    let state = ledger.snapshot("tenant", "agent").unwrap();
    assert_eq!((state.reserved_minor, state.available_minor), (100, 0));
}

#[test]
fn idempotency_and_irreversible_commit_are_enforced() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = FinOpsLedger::open(dir.path().join("finops.sqlite")).unwrap();
    ledger.configure_budget("t", "a", 100).unwrap();
    ledger.reserve("t", "a", "once", 50).unwrap();
    assert_eq!(
        ledger.reserve("t", "a", "once", 50).unwrap(),
        ReservationState::Reserved
    );
    assert!(matches!(
        ledger.reserve("t", "a", "once", 49),
        Err(FinOpsError::IdempotencyConflict)
    ));
    assert_eq!(
        ledger.commit("t", "a", "once").unwrap(),
        ReservationState::Committed
    );
    assert_eq!(
        ledger.commit("t", "a", "once").unwrap(),
        ReservationState::Committed
    );
    assert_eq!(
        ledger.reserve("t", "a", "once", 50).unwrap(),
        ReservationState::Committed
    );
    assert!(matches!(
        ledger.release("t", "a", "once"),
        Err(FinOpsError::InvalidTransition)
    ));
    assert!(matches!(
        ledger.configure_budget("t", "a", 49),
        Err(FinOpsError::BudgetBelowUsage)
    ));
    assert_eq!(ledger.snapshot("t", "a").unwrap().committed_minor, 50);
}

#[test]
fn release_is_idempotent_and_cannot_be_replayed() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = FinOpsLedger::open(dir.path().join("finops.sqlite")).unwrap();
    ledger.configure_budget("t", "a", 100).unwrap();
    ledger.reserve("t", "a", "released", 90).unwrap();
    assert_eq!(
        ledger.release("t", "a", "released").unwrap(),
        ReservationState::Released
    );
    assert_eq!(
        ledger.release("t", "a", "released").unwrap(),
        ReservationState::Released
    );
    assert!(matches!(
        ledger.reserve("t", "a", "released", 90),
        Err(FinOpsError::IdempotencyConflict)
    ));
    assert!(matches!(
        ledger.commit("t", "a", "released"),
        Err(FinOpsError::InvalidTransition)
    ));
    ledger.reserve("t", "a", "new", 100).unwrap();
}

#[test]
fn tenant_agent_and_invalid_inputs_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = FinOpsLedger::open(dir.path().join("finops.sqlite")).unwrap();
    ledger.configure_budget("tenant-a", "agent-a", 10).unwrap();
    ledger.configure_budget("tenant-b", "agent-a", 10).unwrap();
    ledger.reserve("tenant-a", "agent-a", "id", 10).unwrap();
    ledger.reserve("tenant-b", "agent-a", "id", 10).unwrap();
    assert!(matches!(
        ledger.reserve("tenant-a", "agent-b", "id", 1),
        Err(FinOpsError::BudgetNotConfigured)
    ));
    assert!(matches!(
        ledger.reserve("tenant-a", "agent-a", "zero", 0),
        Err(FinOpsError::InvalidAmount)
    ));
    assert!(matches!(
        ledger.reserve("tenant-a", "agent-a", "../escape", 1),
        Err(FinOpsError::InvalidIdentifier("request_id"))
    ));
    assert!(matches!(
        ledger.configure_budget("tenant-a", "agent-a", u64::MAX),
        Err(FinOpsError::InvalidAmount)
    ));
}

#[test]
fn state_survives_reopen_and_two_connections_cannot_double_spend() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("finops.sqlite");
    FinOpsLedger::open(&db)
        .unwrap()
        .configure_budget("t", "a", 100)
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|n| {
            let db = db.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let ledger = FinOpsLedger::open(db).unwrap();
                barrier.wait();
                ledger.reserve("t", "a", &format!("r{n}"), 75)
            })
        })
        .collect();
    barrier.wait();
    let outcomes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|r| matches!(r, Err(FinOpsError::BudgetExceeded)))
            .count(),
        1
    );
    let reopened = FinOpsLedger::open(&db).unwrap();
    assert_eq!(reopened.snapshot("t", "a").unwrap().reserved_minor, 75);
}
