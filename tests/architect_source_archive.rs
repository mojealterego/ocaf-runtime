use ocaf_runtime::{
    architect_https_source_fetch::FetchedSource,
    architect_research_evidence::{
        EvidenceError, ResearchEvidence, ResearchEvidenceGate, SourceRecord,
    },
    architect_source_archive::{ArchiveError, SaveResult, SourceArchive},
    om_engine::OmEngine,
    omnidev_workflow::{Outcome, Role, StepContract, WorkflowContract},
};
use openssl::{pkey::PKey, sign::Signer};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

const NOW: u64 = 1_795_480_000;
fn hash(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
fn fixture(id: &str, body: &[u8]) -> FetchedSource {
    FetchedSource {
        source: SourceRecord {
            url: format!("https://example.com/{id}"),
            retrieved_unix: NOW - 20,
            content_sha256: hash(body),
        },
        status: 200,
        body: body.to_vec(),
    }
}

#[test]
fn immutable_bytes_are_persisted_read_back_and_checked_after_restart() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("source.db");
    let expected = fixture("source1", b"verified archived text");
    {
        let archive = SourceArchive::open(&file).unwrap();
        assert_eq!(
            archive.save("tenant-a", &expected).unwrap(),
            SaveResult::New
        );
        assert_eq!(
            archive.save("tenant-a", &expected).unwrap(),
            SaveResult::Replay
        );
        assert_eq!(
            archive.read("tenant-a", &expected.source).unwrap(),
            expected.body
        );
        assert!(matches!(
            archive.read("tenant-b", &expected.source),
            Err(ArchiveError::Missing)
        ));
    }
    let archive = SourceArchive::open(&file).unwrap();
    assert_eq!(
        archive.read("tenant-a", &expected.source).unwrap(),
        expected.body
    );
    let mut conflict = expected.clone();
    conflict.body = b"changed bytes not original".to_vec();
    conflict.source.content_sha256 = hash(&conflict.body);
    assert!(matches!(
        archive.save("tenant-a", &conflict),
        Err(ArchiveError::Conflict)
    ));
}

#[test]
fn invalid_status_tampered_payload_and_hash_are_rejected() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("source.db");
    let archive = SourceArchive::open(&file).unwrap();
    let original = fixture("source1", b"immutable body 123");
    let mut bad = original.clone();
    bad.source.content_sha256 = "a".repeat(64);
    assert!(matches!(
        archive.save("t", &bad),
        Err(ArchiveError::Invalid)
    ));
    bad = original.clone();
    bad.status = 302;
    assert!(matches!(
        archive.save("t", &bad),
        Err(ArchiveError::Invalid)
    ));
    bad = original.clone();
    bad.source.url = "http://localhost/private".to_string();
    assert!(matches!(
        archive.save("t", &bad),
        Err(ArchiveError::Invalid)
    ));
    for host in ["metadata.localhost", "metadata.local", "metadata.localhost."] {
        bad = original.clone();
        bad.source.url = format!("https://{host}/private");
        assert!(matches!(archive.save("t", &bad), Err(ArchiveError::Invalid)));
    }
    archive.save("t", &original).unwrap();
    let db = rusqlite::Connection::open(&file).unwrap();
    db.execute(
        "UPDATE architect_source_archive SET body=X'616263' WHERE tenant='t'",
        [],
    )
    .unwrap();
    assert!(matches!(
        archive.read("t", &original.source),
        Err(ArchiveError::Integrity)
    ));
    assert!(matches!(
        archive.verify_sources("t", std::slice::from_ref(&original.source)),
        Err(ArchiveError::Integrity)
    ));
}

#[test]
fn missing_source_and_wrong_tenant_cannot_pass_archive_verification() {
    let dir = tempdir().unwrap();
    let archive = SourceArchive::open(dir.path().join("archive.db")).unwrap();
    let one = fixture("first", b"alpha content");
    let two = fixture("second", b"beta content");
    archive.save("tenant-a", &one).unwrap();
    assert!(matches!(
        archive.verify_sources("tenant-a", &[one.source.clone(), two.source.clone()]),
        Err(ArchiveError::Missing)
    ));
    assert!(matches!(
        archive.verify_sources("tenant-b", std::slice::from_ref(&one.source)),
        Err(ArchiveError::Missing)
    ));
    archive.save("tenant-a", &two).unwrap();
    archive
        .verify_sources("tenant-a", &[one.source.clone(), two.source.clone()])
        .unwrap();
}

#[test]
fn real_om_engine_rejects_research_without_archived_bytes_then_accepts_signed_evidence() {
    let dir = tempdir().unwrap();
    let archive = SourceArchive::open(dir.path().join("source.db")).unwrap();
    let engine = OmEngine::open(dir.path().join("om.db")).unwrap();
    let key1 = PKey::generate_ed25519().unwrap();
    let key2 = PKey::generate_ed25519().unwrap();
    let key3 = PKey::generate_ed25519().unwrap();
    let gate = ResearchEvidenceGate::open(
        dir.path().join("evidence.db"),
        &key1.raw_public_key().unwrap(),
        &key2.raw_public_key().unwrap(),
        &key3.raw_public_key().unwrap(),
    )
    .unwrap();
    engine
        .register(
            "tenant-a",
            WorkflowContract {
                tenant: "tenant-a".into(),
                workflow_id: "archived-research".into(),
                base_commit_sha: "a".repeat(40),
                deadline_unix: NOW + 600,
                max_cost_minor: 90,
                steps: vec![StepContract {
                    id: "research".into(),
                    role: Role::Architect,
                    prerequisites: vec![],
                    max_attempts: 1,
                    max_cost_minor: 90,
                    lease_seconds: 300,
                }],
            },
        )
        .unwrap();
    engine
        .claim(
            "tenant-a",
            "archived-research",
            Role::Architect,
            "research",
            1,
            "attempt1",
            NOW,
        )
        .unwrap();
    let one = fixture("one", b"actual source 1");
    let two = fixture("two", b"actual source 2");
    let evidence = ResearchEvidence {
        schema_version: 1,
        tenant: "tenant-a".into(),
        workflow_id: "archived-research".into(),
        step: "research".into(),
        role: Role::Architect,
        epoch: 1,
        attempt_id: "attempt1".into(),
        observed_at_unix: NOW + 1,
        actual_cost_minor: 2,
        outcome: Outcome::Passed,
        summary: "Archive-backed signed research evidence fixture".into(),
        sources: vec![one.source.clone(), two.source.clone()],
        review_of_sha256: None,
    };
    let sig = Signer::new_without_digest(&key2)
        .unwrap()
        .sign_oneshot_to_vec(&evidence.signing_message().unwrap())
        .unwrap();
    assert!(matches!(
        gate.submit_with_archive(&engine, &archive, "tenant-a", &evidence, &sig, NOW + 2),
        Err(EvidenceError::Archive(ArchiveError::Missing))
    ));
    archive.save("tenant-a", &one).unwrap();
    archive.save("tenant-a", &two).unwrap();
    let accepted = gate
        .submit_with_archive(&engine, &archive, "tenant-a", &evidence, &sig, NOW + 3)
        .unwrap();
    assert_eq!(accepted.digest_sha256.len(), 64);
    assert_eq!(
        gate.read("tenant-a", "archived-research", "research", 1, "attempt1")
            .unwrap()
            .digest_sha256,
        accepted.digest_sha256
    );
}
