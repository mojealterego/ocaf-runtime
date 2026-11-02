use ocaf_runtime::builder_core::{from_dsl, BuilderProject};
use ocaf_runtime::builder_store::{BuilderStoreError, SqliteBuilderStore};
use rusqlite::Connection;
use tempfile::NamedTempFile;

fn fixture() -> BuilderProject {
    from_dsl(
        include_str!("../apps/agents-builder-web/fixtures/hello.json"),
        "local",
    )
    .unwrap()
}

fn setup() -> (NamedTempFile, SqliteBuilderStore) {
    let file = NamedTempFile::new().unwrap();
    let store = SqliteBuilderStore::open_existing(file.path()).unwrap();
    (file, store)
}

#[test]
fn durable_create_read_and_tenant_scoped_lookup() {
    let (_file, store) = setup();
    let project = fixture();
    let head = store.save(&project, "local", None).unwrap();
    assert_eq!(head.revision, 1);
    assert_eq!(head.sha256.len(), 64);
    let (loaded, pinned) = store.load("local", "hello", Some(&head)).unwrap().unwrap();
    assert_eq!(loaded, project);
    assert_eq!(pinned, head);
    assert!(store.load("another", "hello", None).unwrap().is_none());
    assert_eq!(
        store.save(&project, "another", None).unwrap_err(),
        BuilderStoreError::Denied
    );
}

#[test]
fn revision_cas_prevents_lost_updates_and_replay() {
    let (_file, store) = setup();
    let mut project = fixture();
    let first = store.save(&project, "local", None).unwrap();
    assert_eq!(
        store.save(&project, "local", None).unwrap_err(),
        BuilderStoreError::Conflict
    );
    project.revision = 2;
    let second = store.save(&project, "local", Some(&first)).unwrap();
    assert_eq!(second.revision, 2);
    assert_ne!(first.sha256, second.sha256);
    assert_eq!(
        store.save(&project, "local", Some(&first)).unwrap_err(),
        BuilderStoreError::Conflict
    );
    assert_eq!(
        store.load("local", "hello", Some(&first)).unwrap_err(),
        BuilderStoreError::Conflict
    );
    assert_eq!(
        store.history("local", "hello").unwrap(),
        vec![first, second]
    );
}

#[test]
fn rejects_revision_gaps_and_unauthorized_tool_grants() {
    let (_file, store) = setup();
    let mut project = fixture();
    let head = store.save(&project, "local", None).unwrap();
    project.revision = 3;
    assert_eq!(
        store.save(&project, "local", Some(&head)).unwrap_err(),
        BuilderStoreError::Conflict
    );
    project.revision = 2;
    project.grants.push("tool_undeclared".into());
    assert!(store.save(&project, "local", Some(&head)).is_err());
}

#[test]
fn rejects_tampered_payload_and_missing_pinned_history() {
    let (file, store) = setup();
    let mut project = fixture();
    let first = store.save(&project, "local", None).unwrap();
    project.revision = 2;
    let second = store.save(&project, "local", Some(&first)).unwrap();
    let conn = Connection::open(file.path()).unwrap();
    conn.execute(
        "UPDATE ocaf_builder_revisions_v1 SET payload=?1 WHERE tenant='local' AND project='hello' AND revision=2",
        [b"{}".as_slice()],
    )
    .unwrap();
    assert_eq!(
        store.load("local", "hello", None).unwrap_err(),
        BuilderStoreError::Integrity
    );
    conn.execute(
        "DELETE FROM ocaf_builder_revisions_v1 WHERE tenant='local' AND project='hello' AND revision=2",
        [],
    )
    .unwrap();
    assert_eq!(
        store.load("local", "hello", Some(&second)).unwrap_err(),
        BuilderStoreError::Conflict
    );
}

#[test]
fn rejects_schema_downgrade_and_nonexistent_database() {
    let (file, store) = setup();
    assert!(store.load("local", "hello", None).unwrap().is_none());
    let conn = Connection::open(file.path()).unwrap();
    conn.execute(
        "UPDATE ocaf_builder_schema SET version=2 WHERE subsystem='builder_projects'",
        [],
    )
    .unwrap();
    assert_eq!(
        store.load("local", "hello", None).unwrap_err(),
        BuilderStoreError::Integrity
    );
    let absent = file.path().with_extension("not-there");
    assert!(SqliteBuilderStore::open_existing(&absent).is_err());
}
