use ocaf_runtime::architect_evolution_approval::{
    ApprovalError, ApprovalState, EvolutionApprovalStore, EvolutionCandidate,
};
use openssl::{pkey::PKey, sign::Signer};
use tempfile::tempdir;

const NOW: i64 = 1_740_000_000;
fn sha(byte: char) -> String {
    byte.to_string().repeat(64)
}
fn key() -> PKey<openssl::pkey::Private> {
    PKey::generate_ed25519().unwrap()
}
fn sign(key: &PKey<openssl::pkey::Private>, message: &[u8]) -> Vec<u8> {
    Signer::new_without_digest(key)
        .unwrap()
        .sign_oneshot_to_vec(message)
        .unwrap()
}
fn candidate() -> EvolutionCandidate {
    EvolutionCandidate {
        tenant: "tenant-a".into(),
        fleet: "fleet-1".into(),
        proposal_id: "evo-2026-11".into(),
        candidate_sha256: sha('a'),
        revision_sha256: sha('b'),
        expires_unix: NOW + 600,
    }
}

#[test]
fn exact_signed_operator_consent_is_single_use_and_persists_after_restart() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("approvals.db");
    let key = key();
    let pubkey = key.raw_public_key().unwrap();
    let original = candidate();
    {
        let store = EvolutionApprovalStore::open(&db, &pubkey).unwrap();
        store.stage("tenant-a", original.clone(), NOW).unwrap();
        store.stage("tenant-a", original.clone(), NOW).unwrap(); // identical idempotence
        assert_eq!(
            store
                .snapshot("tenant-a", "fleet-1", "evo-2026-11")
                .unwrap()
                .state,
            ApprovalState::AwaitingOperator
        );
        assert!(matches!(
            store.consume_for_release(
                "tenant-a",
                "fleet-1",
                "evo-2026-11",
                &sha('a'),
                &sha('b'),
                "release-1",
                NOW + 1
            ),
            Err(ApprovalError::Denied)
        ));
        let m = store
            .signing_message("tenant-a", "fleet-1", "evo-2026-11")
            .unwrap();
        let sig = sign(&key, &m);
        store
            .approve("tenant-a", "fleet-1", "evo-2026-11", &sig, NOW + 1)
            .unwrap();
        assert_eq!(
            store
                .snapshot("tenant-a", "fleet-1", "evo-2026-11")
                .unwrap()
                .state,
            ApprovalState::Approved
        );
    }
    let store = EvolutionApprovalStore::open(&db, &pubkey).unwrap();
    assert!(matches!(
        store.consume_for_release(
            "tenant-a",
            "fleet-1",
            "evo-2026-11",
            &sha('a'),
            &sha('c'),
            "release-1",
            NOW + 2
        ),
        Err(ApprovalError::Denied)
    ));
    store
        .consume_for_release(
            "tenant-a",
            "fleet-1",
            "evo-2026-11",
            &sha('a'),
            &sha('b'),
            "release-1",
            NOW + 2,
        )
        .unwrap();
    assert_eq!(
        store
            .snapshot("tenant-a", "fleet-1", "evo-2026-11")
            .unwrap()
            .state,
        ApprovalState::Consumed
    );
    assert!(matches!(
        store.consume_for_release(
            "tenant-a",
            "fleet-1",
            "evo-2026-11",
            &sha('a'),
            &sha('b'),
            "release-1",
            NOW + 2
        ),
        Err(ApprovalError::Replay)
    ));
}

#[test]
fn forged_key_invalid_signature_or_cross_tenant_access_fails_closed() {
    let dir = tempdir().unwrap();
    let good = key();
    let bad = key();
    let store = EvolutionApprovalStore::open(
        dir.path().join("approval.db"),
        &good.raw_public_key().unwrap(),
    )
    .unwrap();
    store.stage("tenant-a", candidate(), NOW).unwrap();
    let msg = store
        .signing_message("tenant-a", "fleet-1", "evo-2026-11")
        .unwrap();
    assert!(matches!(
        store.signing_message("tenant-b", "fleet-1", "evo-2026-11"),
        Err(ApprovalError::NotFound)
    ));
    assert!(matches!(
        store.approve(
            "tenant-b",
            "fleet-1",
            "evo-2026-11",
            &sign(&good, &msg),
            NOW + 1
        ),
        Err(ApprovalError::NotFound)
    ));
    assert!(matches!(
        store.approve(
            "tenant-a",
            "fleet-1",
            "evo-2026-11",
            &sign(&bad, &msg),
            NOW + 1
        ),
        Err(ApprovalError::Signature)
    ));
    assert_eq!(
        store
            .snapshot("tenant-a", "fleet-1", "evo-2026-11")
            .unwrap()
            .state,
        ApprovalState::AwaitingOperator
    );
}

#[test]
fn time_expiry_and_changed_proposals_cannot_be_authorized() {
    let dir = tempdir().unwrap();
    let key = key();
    let store = EvolutionApprovalStore::open(
        dir.path().join("approval.db"),
        &key.raw_public_key().unwrap(),
    )
    .unwrap();
    store.stage("tenant-a", candidate(), NOW).unwrap();
    let mut changed = candidate();
    changed.revision_sha256 = sha('d');
    assert!(matches!(
        store.stage("tenant-a", changed, NOW),
        Err(ApprovalError::Conflict)
    ));
    let msg = store
        .signing_message("tenant-a", "fleet-1", "evo-2026-11")
        .unwrap();
    assert!(matches!(
        store.approve(
            "tenant-a",
            "fleet-1",
            "evo-2026-11",
            &sign(&key, &msg),
            NOW + 601
        ),
        Err(ApprovalError::Expired)
    ));
    store
        .approve(
            "tenant-a",
            "fleet-1",
            "evo-2026-11",
            &sign(&key, &msg),
            NOW + 1,
        )
        .unwrap();
    assert!(matches!(
        store.consume_for_release(
            "tenant-a",
            "fleet-1",
            "evo-2026-11",
            &sha('a'),
            &sha('b'),
            "release",
            NOW + 601
        ),
        Err(ApprovalError::Expired)
    ));
}

#[test]
fn signature_is_bound_to_tenant_fleet_proposal_and_revisions() {
    let dir = tempdir().unwrap();
    let key = key();
    let store = EvolutionApprovalStore::open(
        dir.path().join("approval.db"),
        &key.raw_public_key().unwrap(),
    )
    .unwrap();
    let first = candidate();
    store.stage("tenant-a", first.clone(), NOW).unwrap();
    let mut second = first.clone();
    second.proposal_id = "evo-2026-12".into();
    store.stage("tenant-a", second, NOW).unwrap();
    let one = store
        .signing_message("tenant-a", "fleet-1", "evo-2026-11")
        .unwrap();
    assert!(matches!(
        store.approve(
            "tenant-a",
            "fleet-1",
            "evo-2026-12",
            &sign(&key, &one),
            NOW + 1
        ),
        Err(ApprovalError::Signature)
    ));
    assert_eq!(
        store
            .snapshot("tenant-a", "fleet-1", "evo-2026-12")
            .unwrap()
            .state,
        ApprovalState::AwaitingOperator
    );
}

#[test]
fn rejects_bad_digests_untrusted_identity_and_long_lived_proposals() {
    let dir = tempdir().unwrap();
    let key = key();
    let store = EvolutionApprovalStore::open(
        dir.path().join("approval.db"),
        &key.raw_public_key().unwrap(),
    )
    .unwrap();
    let mut c = candidate();
    c.candidate_sha256 = "not a sha".into();
    assert!(matches!(
        store.stage("tenant-a", c, NOW),
        Err(ApprovalError::Invalid)
    ));
    let mut c = candidate();
    c.tenant = "../untrusted".into();
    assert!(matches!(
        store.stage("tenant-a", c, NOW),
        Err(ApprovalError::Denied)
    ));
    let mut c = candidate();
    c.expires_unix = NOW + 32 * 86_400;
    assert!(matches!(
        store.stage("tenant-a", c, NOW),
        Err(ApprovalError::Invalid)
    ));
}

#[test]
fn changed_pinned_operator_key_cannot_read_or_consume_old_approval() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("approvals.db");
    let trusted = key();
    let attacker = key();
    {
        let store =
            EvolutionApprovalStore::open(&path, &trusted.raw_public_key().unwrap()).unwrap();
        store.stage("tenant-a", candidate(), NOW).unwrap();
        let m = store
            .signing_message("tenant-a", "fleet-1", "evo-2026-11")
            .unwrap();
        store
            .approve(
                "tenant-a",
                "fleet-1",
                "evo-2026-11",
                &sign(&trusted, &m),
                NOW + 1,
            )
            .unwrap();
    }
    let unauthorized =
        EvolutionApprovalStore::open(&path, &attacker.raw_public_key().unwrap()).unwrap();
    assert!(matches!(
        unauthorized.snapshot("tenant-a", "fleet-1", "evo-2026-11"),
        Err(ApprovalError::Signature)
    ));
    assert!(matches!(
        unauthorized.consume_for_release(
            "tenant-a",
            "fleet-1",
            "evo-2026-11",
            &sha('a'),
            &sha('b'),
            "release-2",
            NOW + 2
        ),
        Err(ApprovalError::Signature)
    ));
}

#[test]
fn database_tampering_with_signature_is_detected() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("approvals.db");
    let key = key();
    let store = EvolutionApprovalStore::open(&path, &key.raw_public_key().unwrap()).unwrap();
    store.stage("tenant-a", candidate(), NOW).unwrap();
    let message = store
        .signing_message("tenant-a", "fleet-1", "evo-2026-11")
        .unwrap();
    store
        .approve(
            "tenant-a",
            "fleet-1",
            "evo-2026-11",
            &sign(&key, &message),
            NOW + 1,
        )
        .unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute(
        "UPDATE architect_evolution_approvals SET signature=zeroblob(64)
         WHERE tenant='tenant-a'",
        [],
    )
    .unwrap();
    assert!(matches!(
        store.snapshot("tenant-a", "fleet-1", "evo-2026-11"),
        Err(ApprovalError::Signature)
    ));
    assert!(matches!(
        store.consume_for_release(
            "tenant-a",
            "fleet-1",
            "evo-2026-11",
            &sha('a'),
            &sha('b'),
            "r1",
            NOW + 3
        ),
        Err(ApprovalError::Signature)
    ));
}

#[test]
fn multiple_connections_racing_on_same_release_cannot_consume_twice() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("approvals.db");
    let key = key();
    let pubkey = key.raw_public_key().unwrap();
    let a = EvolutionApprovalStore::open(&path, &pubkey).unwrap();
    let b = EvolutionApprovalStore::open(&path, &pubkey).unwrap();
    a.stage("tenant-a", candidate(), NOW).unwrap();
    let message = a
        .signing_message("tenant-a", "fleet-1", "evo-2026-11")
        .unwrap();
    a.approve(
        "tenant-a",
        "fleet-1",
        "evo-2026-11",
        &sign(&key, &message),
        NOW + 1,
    )
    .unwrap();
    assert!(a
        .consume_for_release(
            "tenant-a",
            "fleet-1",
            "evo-2026-11",
            &sha('a'),
            &sha('b'),
            "r1",
            NOW + 2
        )
        .is_ok());
    assert!(matches!(
        b.consume_for_release(
            "tenant-a",
            "fleet-1",
            "evo-2026-11",
            &sha('a'),
            &sha('b'),
            "r2",
            NOW + 2
        ),
        Err(ApprovalError::Replay)
    ));
}
