use ocaf_runtime::{om_engine::OmEngine, omnidev_workflow::StepStatus};
use std::{path::Path, process::Command};
use tempfile::tempdir;

fn run(schedule: &Path, engine: &Path, sha: &str, budget: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ocaf-architect-scheduler"))
        .arg("--once")
        .env("OCAF_ARCHITECT_DB", schedule)
        .env("OCAF_ARCHITECT_TENANT", "tenant-a")
        .env("OCAF_ARCHITECT_FLEET", "fleet")
        .env("OCAF_ARCHITECT_TIMEZONE", "UTC")
        .env("OCAF_ARCHITECT_OM_DB", engine)
        .env("OCAF_ARCHITECT_BASE_COMMIT_SHA", sha)
        .env("OCAF_ARCHITECT_MAX_COST_MINOR", budget)
        .output()
        .unwrap()
}

#[test]
fn live_cli_creates_three_real_om_workflow_contracts_and_recovers_restart() {
    let dir = tempdir().unwrap();
    let q = dir.path().join("schedule.db");
    let om = dir.path().join("om.db");
    let sha = "c".repeat(40);
    let first = run(&q, &om, &sha, "90");
    assert!(
        first.status.success(),
        "{:?}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(String::from_utf8_lossy(&first.stdout).contains("om_workflows=3"));
    let db = rusqlite::Connection::open(&om).unwrap();
    let contracts: Vec<(String, String)> = {
        let mut stmt = db
            .prepare("SELECT workflow_id,contract_json FROM om_workflows ORDER BY workflow_id")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert_eq!(contracts.len(), 3);
    assert!(contracts.iter().all(|(_, v)| v.contains(&sha)));
    drop(db);
    let engine = OmEngine::open(&om).unwrap();
    for (id, _) in contracts {
        assert!(matches!(
            engine.status("tenant-a", &id, "plan"),
            Ok(StepStatus::Ready)
        ));
        assert_eq!(engine.audit_head("tenant-a", &id).unwrap().events, 0);
    }
    let second = run(&q, &om, &sha, "90");
    assert!(second.status.success());
    assert!(String::from_utf8_lossy(&second.stdout).contains("new_runs=0"));
    assert!(String::from_utf8_lossy(&second.stdout).contains("om_workflows=3"));
}

#[test]
fn invalid_policy_cannot_create_either_database() {
    let dir = tempdir().unwrap();
    let q = dir.path().join("schedule.db");
    let om = dir.path().join("om.db");
    let bad = run(&q, &om, "not-a-commit", "90");
    assert!(!bad.status.success());
    assert!(!q.exists());
    assert!(!om.exists());
    let bad_budget = run(&q, &om, &"c".repeat(40), "2");
    assert!(!bad_budget.status.success());
    assert!(!q.exists());
    assert!(!om.exists());
}

#[test]
fn pinned_revision_change_is_fail_closed_after_existing_registration() {
    let dir = tempdir().unwrap();
    let q = dir.path().join("schedule.db");
    let om = dir.path().join("om.db");
    assert!(run(&q, &om, &"a".repeat(40), "90").status.success());
    let conflict = run(&q, &om, &"b".repeat(40), "90");
    assert!(!conflict.status.success());
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("contract"));
}
