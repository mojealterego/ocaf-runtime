use ocaf_runtime::architect_research_schedule::{
    ArchitectResearchScheduler, ResearchError, ResearchKind, RunState,
};
use tempfile::tempdir;

// UTC timestamps supplied by trusted host, not the client.
const FEB29_2024: i64 = 1_709_208_000;
const APR03_2024: i64 = 1_712_145_600;
const LONG_OUTAGE: i64 = FEB29_2024 + 95 * 86_400;

#[test]
fn first_tick_enqueues_three_distinct_periods_and_deduplicates() {
    let dir = tempdir().unwrap();
    let q = ArchitectResearchScheduler::open(dir.path().join("runs.db")).unwrap();
    let first = q.tick("tenant_a", "fleet", FEB29_2024).unwrap();
    assert_eq!(first.new_runs, 3);
    assert_eq!(first.through_unix, FEB29_2024);
    assert_eq!(
        q.list_due("tenant_a", "fleet", FEB29_2024, 64)
            .unwrap()
            .len(),
        3
    );
    assert_eq!(q.tick("tenant_a", "fleet", FEB29_2024).unwrap().new_runs, 0);
    let due = q.list_due("tenant_a", "fleet", FEB29_2024, 64).unwrap();
    // Oldest period first: the monthly period starts on February 1,
    // before the weekly period that starts on February 26.
    assert_eq!(due[0].kind, ResearchKind::SystemEvolution);
    assert_eq!(due[1].kind, ResearchKind::Competitive);
    assert_eq!(due[2].kind, ResearchKind::Market);
    assert_eq!(
        q.list_due("tenant_b", "fleet", FEB29_2024, 64)
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn restart_recovers_all_weekly_and_monthly_periods_transactionally() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("runs.db");
    ArchitectResearchScheduler::open(&db)
        .unwrap()
        .tick("tenant", "fleet", FEB29_2024)
        .unwrap();
    let q = ArchitectResearchScheduler::open(&db).unwrap();
    let r = q.tick("tenant", "fleet", APR03_2024).unwrap();
    assert_eq!(r.from_unix, FEB29_2024);
    assert_eq!(r.through_unix, APR03_2024);
    // Week starts: Feb 26, Mar 4,11,18,25, Apr 1 => 6 weeks total.
    // Calendar months: February, March, April => 3 months total.
    assert_eq!(r.new_runs, 12); // 5 new weeks * 2 kinds + 2 new months
    let due = q.list_due("tenant", "fleet", APR03_2024, 64).unwrap();
    assert_eq!(due.len(), 15);
    assert_eq!(q.tick("tenant", "fleet", APR03_2024).unwrap().new_runs, 0);
}

#[test]
fn excessive_outage_fails_closed_without_advancing_watermark() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("runs.db");
    let q = ArchitectResearchScheduler::open(&path).unwrap();
    q.tick("a", "fleet", FEB29_2024).unwrap();
    assert!(matches!(
        q.tick("a", "fleet", LONG_OUTAGE),
        Err(ResearchError::Capacity)
    ));
    let due = q.list_due("a", "fleet", LONG_OUTAGE, 64).unwrap();
    assert_eq!(due.len(), 3);
    // A failed scan must not advance the cursor.  An intervening catch-up
    // within its original limit must remain possible.
    let next = q.tick("a", "fleet", APR03_2024).unwrap();
    assert_eq!(next.from_unix, FEB29_2024);
}

#[test]
fn clock_rollback_and_unbounded_due_queries_are_rejected() {
    let dir = tempdir().unwrap();
    let q = ArchitectResearchScheduler::open(dir.path().join("q.db")).unwrap();
    q.tick("a", "f", APR03_2024).unwrap();
    assert!(matches!(
        q.tick("a", "f", FEB29_2024),
        Err(ResearchError::Invalid)
    ));
    assert!(matches!(
        q.list_due("a", "f", APR03_2024, 0),
        Err(ResearchError::Invalid)
    ));
    assert!(matches!(
        q.list_due("a", "f", APR03_2024, 65),
        Err(ResearchError::Invalid)
    ));
    assert!(matches!(
        q.tick("../escape", "f", APR03_2024),
        Err(ResearchError::Invalid)
    ));
}

#[test]
fn overdue_scan_preserves_active_leases_and_completed_results() {
    let dir = tempdir().unwrap();
    let q = ArchitectResearchScheduler::open(dir.path().join("q.db")).unwrap();
    let initial = q.tick("a", "f", FEB29_2024).unwrap();
    assert_eq!(initial.new_runs, 3);
    let due = q.list_due("a", "f", FEB29_2024, 64).unwrap();
    let p = &due[0];
    let _lease = q
        .claim("a", "f", p.kind, p.start_unix, "worker", FEB29_2024, 300)
        .unwrap();
    q.tick("a", "f", APR03_2024).unwrap();
    assert_eq!(
        q.snapshot("a", "f", p.kind, p.start_unix).unwrap().state,
        RunState::Leased
    );
    let pending = q.list_due("a", "f", APR03_2024, 64).unwrap();
    assert!(!pending.contains(p));
}

#[test]
fn calendar_year_rollover_and_competing_ticks_are_idempotent() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("q.db");
    let first = ArchitectResearchScheduler::open(&path).unwrap();
    let second = ArchitectResearchScheduler::open(&path).unwrap();
    // 2024-12-30 00:00Z is Monday, 2025-01-02 belongs to the same ISO week.
    let dec30 = 1_735_516_800;
    let jan02 = 1_735_776_000;
    assert_eq!(first.tick("a", "fleet", dec30).unwrap().new_runs, 3);
    assert_eq!(second.tick("a", "fleet", dec30).unwrap().new_runs, 0);
    assert_eq!(second.tick("a", "fleet", jan02).unwrap().new_runs, 1);
    assert_eq!(first.tick("a", "fleet", jan02).unwrap().new_runs, 0);
    assert_eq!(second.list_due("a", "fleet", jan02, 64).unwrap().len(), 4);
}
