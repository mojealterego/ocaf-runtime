//! Stage 0.43 durable OMAs agent registry integration tests.
use ocaf_runtime::omas_server::{AgentRegistrationRequest, OmasServerConfig};
use ocaf_runtime::stage043_agent_store::Stage043AgentStore;
use std::os::unix::fs::PermissionsExt;

fn prepare_db(dir: &tempfile::TempDir, path: &std::path::Path) {
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    rusqlite::Connection::open(path).unwrap().close().unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
}


fn request(tenant: &str, agent: &str, id: &str) -> AgentRegistrationRequest {
    AgentRegistrationRequest {
        request_id: id.into(),
        agent_id: agent.into(),
        tenant: tenant.into(),
        manifest_digest: format!("sha256:{}", "a".repeat(64)),
        declared_state: "STAGED".into(),
        readiness: false,
        ostc_enabled: false,
        source_access: "disabled".into(),
    }
}

#[test]
fn durable_agents_survive_restart_and_replay_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agents.sqlite");
    prepare_db(&dir, &path);
    let store = Stage043AgentStore::open(&path, OmasServerConfig::default()).unwrap();
    assert!(
        store
            .admit(request("tenant_a", "agent1", "nonce1"))
            .unwrap()
            .admitted
    );
    drop(store);
    let reopened = Stage043AgentStore::open(&path, OmasServerConfig::default()).unwrap();
    assert_eq!(reopened.registered_count().unwrap(), 1);
    assert_eq!(
        reopened
            .admit(request("tenant_a", "agent1", "nonce1"))
            .unwrap()
            .reason,
        "replay_request_id"
    );
    assert!(
        reopened
            .admit(request("tenant_b", "agent1", "nonce2"))
            .unwrap()
            .admitted
    );
    assert_eq!(reopened.registered_count().unwrap(), 2);
}

#[test]
fn capacity_denial_does_not_consume_request_id() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agents.sqlite");
    prepare_db(&dir, &path);
    let config = OmasServerConfig {
        max_agents: 1,
        ..Default::default()
    };
    let store = Stage043AgentStore::open(&path, config).unwrap();
    assert!(
        store
            .admit(request("tenant_a", "agent1", "first"))
            .unwrap()
            .admitted
    );
    assert_eq!(
        store
            .admit(request("tenant_b", "agent2", "second"))
            .unwrap()
            .reason,
        "agent_capacity_exhausted"
    );
    assert_eq!(store.registered_count().unwrap(), 1);
}

#[test]
fn malformed_or_active_registration_never_mutates_store() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agents.sqlite");
    prepare_db(&dir, &path);
    let store = Stage043AgentStore::open(&path, OmasServerConfig::default()).unwrap();
    let mut active = request("tenant_a", "agent1", "nonce1");
    active.declared_state = "ACTIVE".into();
    assert!(!store.admit(active).unwrap().admitted);
    assert_eq!(store.registered_count().unwrap(), 0);
}

#[test]
fn agent_store_rejects_public_directory_file_symlink_and_hardlink() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agents.sqlite");
    prepare_db(&dir, &path);
    let config = OmasServerConfig::default();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Stage043AgentStore::open(&path, config.clone()).is_err());
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(Stage043AgentStore::open(&path, config.clone()).is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let alias = dir.path().join("alias.sqlite");
    std::os::unix::fs::symlink(&path, &alias).unwrap();
    assert!(Stage043AgentStore::open(&alias, config.clone()).is_err());
    std::fs::hard_link(&path, dir.path().join("linked.sqlite")).unwrap();
    assert!(Stage043AgentStore::open(&path, config).is_err());
}
