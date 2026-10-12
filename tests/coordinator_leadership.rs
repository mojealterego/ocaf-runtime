use ocaf_runtime::coordinator::leadership::{LeaderGuard, LeadershipError};
use std::fs;
use std::time::Duration;

#[test]
fn single_active_coordinator_and_takeover_after_release() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coordinator.lock");
    let leader = LeaderGuard::try_acquire(&path).unwrap().unwrap();
    assert!(LeaderGuard::try_acquire(&path).unwrap().is_none());
    drop(leader);
    let successor = LeaderGuard::try_acquire(&path).unwrap().unwrap();
    assert!(LeaderGuard::try_acquire(&path).unwrap().is_none());
    drop(successor);
}

#[tokio::test]
async fn standby_promotes_after_leader_crashes_and_retains_job_journal() {
    use ocaf_runtime::jobs::{JobSpec, JobStatus, JobStore};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coordinator.lock");
    let db = dir.path().join("jobs.sqlite");
    let primary = LeaderGuard::try_acquire(&path).unwrap().unwrap();
    let store = JobStore::open(&db).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    store
        .submit(JobSpec {
            id: id.clone(),
            tool: "echo".into(),
            args: serde_json::json!({"message": "survives"}),
            retry_safe: false,
            max_attempts: 1,
        })
        .await
        .unwrap();
    drop(store);

    let standby_path = path.clone();
    let join = tokio::spawn(async move {
        LeaderGuard::acquire_standby(&standby_path, Duration::from_millis(20)).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(100), async {
        while !join.is_finished() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .is_err());
    drop(primary);
    let standby = tokio::time::timeout(Duration::from_secs(2), join)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(LeaderGuard::try_acquire(&path).unwrap().is_none());
    assert_eq!(
        JobStore::open(&db)
            .unwrap()
            .get(&id)
            .await
            .unwrap()
            .unwrap()
            .status,
        JobStatus::Pending
    );
    drop(standby);
}

#[test]
fn leader_rejects_invalid_paths_and_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        LeaderGuard::try_acquire(std::path::Path::new("relative.lock")),
        Err(LeadershipError::InvalidPath)
    ));
    assert!(matches!(
        LeaderGuard::try_acquire(&dir.path().join("missing").join("leader.lock")),
        Err(LeadershipError::Io)
    ));
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let target = dir.path().join("real");
        fs::write(&target, "sensitive").unwrap();
        let path = dir.path().join("link");
        symlink(&target, &path).unwrap();
        assert!(LeaderGuard::try_acquire(&path).is_err());
    }
}

#[cfg(unix)]
#[test]
fn leader_lock_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lock");
    let _lock = LeaderGuard::try_acquire(&path).unwrap().unwrap();
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[cfg(unix)]
#[test]
fn child_holds_leader_lock_for_crash_test() {
    let Ok(lock_file) = std::env::var("OCAF_CHILD_LOCK_FILE") else {
        return;
    };
    let ready = std::env::var("OCAF_CHILD_LOCK_READY").unwrap();
    let _guard = LeaderGuard::try_acquire(std::path::Path::new(&lock_file))
        .unwrap()
        .unwrap();
    fs::write(ready, b"locked").unwrap();
    std::thread::sleep(Duration::from_secs(20));
}

#[cfg(unix)]
#[test]
fn killed_coordinator_process_releases_kernel_lock_for_standby() {
    use std::process::{Command, Stdio};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coordinator.lock");
    let ready = dir.path().join("child-ready");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("child_holds_leader_lock_for_crash_test")
        .arg("--nocapture")
        .env("OCAF_CHILD_LOCK_FILE", &path)
        .env("OCAF_CHILD_LOCK_READY", &ready)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut acquired = false;
    for _ in 0..200 {
        if ready.is_file() {
            acquired = true;
            break;
        }
        if child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let blocked = LeaderGuard::try_acquire(&path).unwrap().is_none();
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(
        acquired && blocked,
        "child failed to obtain the original exclusive lock"
    );
    let standby = LeaderGuard::try_acquire(&path).unwrap();
    assert!(
        standby.is_some(),
        "kernel lock was not released after process death"
    );
}
