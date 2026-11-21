use ocaf_runtime::architect_research_schedule::{
    ArchitectResearchScheduler, Recorded, ResearchError, ResearchKind, RunState,
};
use ocaf_runtime::omnidev_workflow::Role;
use tempfile::tempdir;

const NOW: i64 = 1_700_000_000;

fn evidence() -> String {
    "a".repeat(64)
}

#[test]
fn two_weekly_one_monthly_and_idempotent_registration() {
    let dir = tempdir().unwrap();
    let scheduler = ArchitectResearchScheduler::open(dir.path().join("research.sqlite")).unwrap();
    let periods = scheduler
        .enqueue_current("tenant", "fleet", 1_709_208_000)
        .unwrap();
    assert_eq!(periods[0].kind, ResearchKind::Competitive);
    assert_eq!(periods[1].kind, ResearchKind::Market);
    assert_eq!(periods[2].kind, ResearchKind::SystemEvolution);
    assert_eq!(periods[0].start_unix, periods[1].start_unix);
    assert_eq!(periods[0].end_unix - periods[0].start_unix, 7 * 86_400);
    assert_eq!(periods[2].end_unix - periods[2].start_unix, 29 * 86_400);

    let claim = scheduler
        .claim(
            "tenant",
            "fleet",
            periods[0].kind,
            periods[0].start_unix,
            "worker",
            1_709_208_001,
            60,
        )
        .unwrap();
    assert_eq!(claim.epoch, 1);
    assert_eq!(
        scheduler
            .enqueue_current("tenant", "fleet", 1_709_208_002)
            .unwrap(),
        periods,
    );
    assert_eq!(
        scheduler
            .snapshot("tenant", "fleet", periods[0].kind, periods[0].start_unix)
            .unwrap()
            .state,
        RunState::Leased
    );
}

#[test]
fn persists_across_restart_fences_tenants_and_replays_receipts() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("research.sqlite");
    let period = {
        let scheduler = ArchitectResearchScheduler::open(&path).unwrap();
        scheduler.enqueue_current("alice", "f1", NOW).unwrap()[0].clone()
    };
    let scheduler = ArchitectResearchScheduler::open(&path).unwrap();
    assert!(matches!(
        scheduler.snapshot("bob", "f1", period.kind, period.start_unix),
        Err(ResearchError::NotFound)
    ));
    let lease = scheduler
        .claim(
            "alice",
            "f1",
            period.kind,
            period.start_unix,
            "w1",
            NOW + 1,
            10,
        )
        .unwrap();
    assert!(matches!(
        scheduler.claim(
            "alice",
            "f1",
            period.kind,
            period.start_unix,
            "w2",
            NOW + 2,
            10
        ),
        Err(ResearchError::Busy)
    ));
    assert!(matches!(
        scheduler.complete("bob", "f1", &lease, true, &evidence(), NOW + 2),
        Err(ResearchError::NotFound)
    ));
    assert_eq!(
        scheduler
            .complete("alice", "f1", &lease, true, &evidence(), NOW + 2)
            .unwrap(),
        Recorded::New
    );
    assert_eq!(
        scheduler
            .complete("alice", "f1", &lease, true, &evidence(), NOW + 2)
            .unwrap(),
        Recorded::Replay
    );
    assert!(matches!(
        scheduler.complete("alice", "f1", &lease, false, &evidence(), NOW + 2),
        Err(ResearchError::Fenced)
    ));
    drop(scheduler);
    let reopened = ArchitectResearchScheduler::open(&path).unwrap();
    let status = reopened
        .snapshot("alice", "f1", period.kind, period.start_unix)
        .unwrap();
    assert_eq!(status.state, RunState::Succeeded);
    assert_eq!(
        status.last_evidence_sha256.as_deref(),
        Some(evidence().as_str())
    );
}

#[test]
fn expired_leases_never_retry_without_reconciliation() {
    let dir = tempdir().unwrap();
    let scheduler = ArchitectResearchScheduler::open(dir.path().join("q.sqlite")).unwrap();
    let p = scheduler.enqueue_current("alice", "fleet", NOW).unwrap()[0].clone();
    let lease = scheduler
        .claim(
            "alice",
            "fleet",
            p.kind,
            p.start_unix,
            "worker1",
            NOW + 1,
            5,
        )
        .unwrap();
    assert!(matches!(
        scheduler.complete("alice", "fleet", &lease, true, &evidence(), NOW + 8),
        Err(ResearchError::Uncertain)
    ));
    assert!(matches!(
        scheduler.claim(
            "alice",
            "fleet",
            p.kind,
            p.start_unix,
            "worker2",
            NOW + 8,
            5
        ),
        Err(ResearchError::Uncertain)
    ));
    assert!(matches!(
        scheduler.reconcile_expired(
            Role::Verifier,
            "alice",
            "fleet",
            p.kind,
            p.start_unix,
            lease.epoch,
            true,
            &evidence(),
            NOW + 6,
        ),
        Err(ResearchError::Busy)
    ));
    assert!(matches!(
        scheduler.reconcile_expired(
            Role::Planner,
            "alice",
            "fleet",
            p.kind,
            p.start_unix,
            lease.epoch,
            true,
            &evidence(),
            NOW + 8
        ),
        Err(ResearchError::Denied)
    ));
    scheduler
        .reconcile_expired(
            Role::Verifier,
            "alice",
            "fleet",
            p.kind,
            p.start_unix,
            lease.epoch,
            true,
            &evidence(),
            NOW + 8,
        )
        .unwrap();
    let next = scheduler
        .claim(
            "alice",
            "fleet",
            p.kind,
            p.start_unix,
            "worker2",
            NOW + 9,
            5,
        )
        .unwrap();
    assert_eq!(next.epoch, lease.epoch + 1);
    assert!(matches!(
        scheduler.complete("alice", "fleet", &lease, true, &evidence(), NOW + 10),
        Err(ResearchError::Fenced)
    ));
    assert_eq!(
        scheduler
            .complete("alice", "fleet", &next, true, &evidence(), NOW + 10)
            .unwrap(),
        Recorded::New
    );
}

#[test]
fn failed_attempts_exhaust_budget_and_uncertain_effect_halts() {
    let dir = tempdir().unwrap();
    let scheduler = ArchitectResearchScheduler::open(dir.path().join("q.sqlite")).unwrap();
    let runs = scheduler.enqueue_current("alice", "fleet", NOW).unwrap();
    let p = &runs[1];
    for attempt in 1..=3 {
        let lease = scheduler
            .claim(
                "alice",
                "fleet",
                p.kind,
                p.start_unix,
                "worker",
                NOW + 10 * attempt,
                5,
            )
            .unwrap();
        assert_eq!(lease.attempt, attempt);
        scheduler
            .complete(
                "alice",
                "fleet",
                &lease,
                false,
                &evidence(),
                NOW + 10 * attempt + 1,
            )
            .unwrap();
    }
    assert_eq!(
        scheduler
            .snapshot("alice", "fleet", p.kind, p.start_unix)
            .unwrap()
            .state,
        RunState::Halted
    );
    assert!(scheduler
        .claim(
            "alice",
            "fleet",
            p.kind,
            p.start_unix,
            "worker",
            NOW + 100,
            5
        )
        .is_err());

    let p = &runs[2];
    let lease = scheduler
        .claim("alice", "fleet", p.kind, p.start_unix, "w", NOW + 1, 5)
        .unwrap();
    scheduler
        .reconcile_expired(
            Role::Verifier,
            "alice",
            "fleet",
            p.kind,
            p.start_unix,
            lease.epoch,
            false,
            &evidence(),
            NOW + 8,
        )
        .unwrap();
    assert_eq!(
        scheduler
            .snapshot("alice", "fleet", p.kind, p.start_unix)
            .unwrap()
            .state,
        RunState::Halted
    );
}

#[test]
fn competing_sqlite_connections_claim_once() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("q.sqlite");
    let first = ArchitectResearchScheduler::open(&path).unwrap();
    let second = ArchitectResearchScheduler::open(&path).unwrap();
    let p = first.enqueue_current("alice", "fleet", NOW).unwrap()[0].clone();
    let a = first.claim("alice", "fleet", p.kind, p.start_unix, "a", NOW, 10);
    let b = second.claim("alice", "fleet", p.kind, p.start_unix, "b", NOW, 10);
    assert!(a.is_ok());
    assert!(matches!(b, Err(ResearchError::Busy)));
}

#[test]
fn invalid_identity_evidence_lease_and_symlink_are_denied() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("q.sqlite");
    let scheduler = ArchitectResearchScheduler::open(&path).unwrap();
    assert!(matches!(
        scheduler.enqueue_current("../other", "fleet", NOW),
        Err(ResearchError::Invalid)
    ));
    let p = scheduler.enqueue_current("tenant", "fleet", NOW).unwrap()[0].clone();
    assert!(matches!(
        scheduler.claim("tenant", "fleet", p.kind, p.start_unix, "worker", NOW, 3601),
        Err(ResearchError::Invalid)
    ));
    let lease = scheduler
        .claim("tenant", "fleet", p.kind, p.start_unix, "worker", NOW, 5)
        .unwrap();
    assert!(matches!(
        scheduler.complete("tenant", "fleet", &lease, true, "not-a-digest", NOW + 1),
        Err(ResearchError::Invalid)
    ));
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let link = dir.path().join("bad.sqlite");
        symlink(&path, &link).unwrap();
        assert!(matches!(
            ArchitectResearchScheduler::open(link),
            Err(ResearchError::Invalid)
        ));
    }
}
