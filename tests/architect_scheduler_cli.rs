use ocaf_runtime::architect_research_schedule::ArchitectResearchScheduler;
use std::{path::Path, process::Command};
use tempfile::tempdir;

fn run_once(db: &Path, tenant: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ocaf-architect-scheduler"))
        .arg("--once")
        .env("OCAF_ARCHITECT_DB", db)
        .env("OCAF_ARCHITECT_TENANT", tenant)
        .env("OCAF_ARCHITECT_FLEET", "fleet-1")
        .env("OCAF_ARCHITECT_TIMEZONE", "UTC")
        .env("OCAF_ARCHITECT_INTERVAL_SECONDS", "60")
        .output()
        .unwrap()
}

#[test]
fn actual_scheduler_executable_ticks_database_without_external_effects() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("runs.db");
    let first = run_once(&path, "tenant-a");
    assert!(
        first.status.success(),
        "{:?}",
        String::from_utf8_lossy(&first.stderr)
    );
    let stdout = String::from_utf8_lossy(&first.stdout);
    assert!(stdout.contains("new_runs=3"), "{stdout}");
    let second = run_once(&path, "tenant-a");
    assert!(second.status.success());
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(stdout.contains("new_runs=0"), "{stdout}");
    let store = ArchitectResearchScheduler::open(&path).unwrap();
    // Wide time bound: all registered jobs are now due.
    let due = store
        .list_due("tenant-a", "fleet-1", 4_000_000_000, 64)
        .unwrap();
    assert_eq!(due.len(), 3);
}

#[test]
fn invalid_tenant_and_non_utc_timezone_are_denied() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("runs.db");
    let invalid = run_once(&path, "../tenant");
    assert!(!invalid.status.success());
    assert!(!path.exists());

    let wrong_tz = Command::new(env!("CARGO_BIN_EXE_ocaf-architect-scheduler"))
        .arg("--once")
        .env("OCAF_ARCHITECT_DB", &path)
        .env("OCAF_ARCHITECT_TENANT", "tenant-a")
        .env("OCAF_ARCHITECT_FLEET", "fleet-1")
        .env("OCAF_ARCHITECT_TIMEZONE", "Europe/Warsaw")
        .output()
        .unwrap();
    assert!(!wrong_tz.status.success());
    assert!(!path.exists());
}

#[test]
fn unsupported_arguments_fail_before_creating_database() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("runs.db");
    let result = Command::new(env!("CARGO_BIN_EXE_ocaf-architect-scheduler"))
        .arg("--unsafe-commit")
        .env("OCAF_ARCHITECT_DB", &path)
        .env("OCAF_ARCHITECT_TENANT", "tenant-a")
        .env("OCAF_ARCHITECT_FLEET", "fleet-1")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!path.exists());
}
