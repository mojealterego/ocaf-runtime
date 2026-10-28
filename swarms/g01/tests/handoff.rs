use ocaf_g01_swarm::{ClaimRequest, ReceiptPayload, SignedReceipt, SwarmStore, SWARM_ID};
use ocaf_stage_ledger::{StageRecord, StageStatus};
use openssl::pkey::PKey;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

fn stage(id: &str, dependencies: &[&str]) -> StageRecord {
    StageRecord {
        stage_id: id.into(),
        dependencies: dependencies.iter().map(|id| (*id).into()).collect(),
        status: StageStatus::Ready,
        claimed_by: None,
    }
}

fn plan() -> Vec<StageRecord> {
    vec![stage("0.2", &["0.1"]), stage("0.1", &[])]
}

fn request() -> ClaimRequest {
    ClaimRequest {
        swarm_id: SWARM_ID.into(),
        declared_state: "IN_DEVELOPMENT".into(),
        readiness: false,
        ostc_enabled: false,
        source_access: "disabled".into(),
    }
}

fn setup() -> (TempDir, SwarmStore, [u8; 32]) {
    let dir = TempDir::new().unwrap();
    let store = SwarmStore::new(dir.path().join("state"));
    let key = PKey::generate_ed25519().unwrap();
    let private: [u8; 32] = key.raw_private_key().unwrap().try_into().unwrap();
    store
        .initialize(
            plan(),
            &ocaf_g01_swarm::encode_hex(&key.raw_public_key().unwrap()),
        )
        .unwrap();
    (dir, store, private)
}

fn prepare(store: &SwarmStore, artifact: &Path, id: &str) -> ReceiptPayload {
    store
        .prepare(
            artifact,
            "a".repeat(40),
            "cargo test -p ocaf-g01-swarm --locked".into(),
            "PASS".into(),
            id.into(),
            vec!["Local test fixture; no live Zenoh".into()],
        )
        .unwrap()
}

fn sign(
    store: &SwarmStore,
    artifact: &Path,
    payload: ReceiptPayload,
    private: &[u8; 32],
) -> SignedReceipt {
    store.operator_sign(artifact, payload, private).unwrap()
}

#[test]
fn dependency_order_persists_across_restart_and_requires_operator_handoff() {
    let (dir, store, private) = setup();
    let first = dir.path().join("build-1.bin");
    let second = dir.path().join("build-2.bin");
    fs::write(&first, b"first compiled artifact").unwrap();
    fs::write(&second, b"second compiled artifact").unwrap();
    assert_eq!(store.next().unwrap().unwrap().stage_id, "0.1");
    assert_eq!(store.claim(request()).unwrap(), "0.1");
    assert!(store.next().unwrap().is_none());
    assert!(store.claim(request()).is_err());
    let payload = prepare(&store, &first, "receipt-1");
    assert!(store
        .status()
        .unwrap()
        .iter()
        .all(|s| s.status != StageStatus::CompleteVerified));
    let signed = sign(&store, &first, payload, &private);
    assert_eq!(store.handoff(&first, signed).unwrap(), "0.1");

    let restarted = SwarmStore::new(dir.path().join("state"));
    assert_eq!(restarted.next().unwrap().unwrap().stage_id, "0.2");
    assert_eq!(restarted.claim(request()).unwrap(), "0.2");
    let payload = prepare(&restarted, &second, "receipt-2");
    let signed = sign(&restarted, &second, payload, &private);
    assert_eq!(restarted.handoff(&second, signed).unwrap(), "0.2");
    assert!(restarted.next().unwrap().is_none());
    assert_eq!(
        restarted
            .status()
            .unwrap()
            .iter()
            .filter(|s| s.status == StageStatus::CompleteVerified)
            .count(),
        2
    );
}

#[test]
fn forged_signature_payload_mutation_and_artifact_substitution_do_not_advance() {
    let (dir, store, private) = setup();
    let artifact = dir.path().join("artifact.bin");
    let substitute = dir.path().join("substitute.bin");
    fs::write(&artifact, b"actual bytes").unwrap();
    fs::write(&substitute, b"different bytes").unwrap();
    store.claim(request()).unwrap();
    let payload = prepare(&store, &artifact, "receipt-1");
    let signed = sign(&store, &artifact, payload, &private);

    let mut altered = signed.clone();
    altered.payload.receipt.source_commit = "b".repeat(40);
    assert!(store.handoff(&artifact, altered).is_err());
    let mut forged = signed.clone();
    forged.signature_hex = "00".repeat(64);
    assert!(store.handoff(&artifact, forged).is_err());
    assert!(store.handoff(&substitute, signed.clone()).is_err());
    assert_eq!(store.status().unwrap()[0].status, StageStatus::Claimed);
    assert_eq!(store.handoff(&artifact, signed.clone()).unwrap(), "0.1");
    assert!(store.handoff(&artifact, signed).is_err());
}

#[test]
fn active_readiness_ostc_and_source_access_are_denied() {
    let (_dir, store, _private) = setup();
    let mut active = request();
    active.declared_state = "ACTIVE".into();
    assert!(store.claim(active).is_err());
    let mut ready = request();
    ready.readiness = true;
    assert!(store.claim(ready).is_err());
    let mut ostc = request();
    ostc.ostc_enabled = true;
    assert!(store.claim(ostc).is_err());
    let mut source = request();
    source.source_access = "enabled".into();
    assert!(store.claim(source).is_err());
    let mut other = request();
    other.swarm_id = "OCAF-SWARM-02".into();
    assert!(store.claim(other).is_err());
    assert_eq!(store.next().unwrap().unwrap().stage_id, "0.1");
    assert!(serde_json::from_str::<ClaimRequest>(
        r#"{"swarm_id":"OCAF-SWARM-01","declared_state":"IN_DEVELOPMENT","readiness":false,"ostc_enabled":false,"source_access":"disabled","override":true}"#
    ).is_err());
}

#[test]
fn failed_test_and_wrong_operator_key_cannot_make_a_receipt() {
    let (dir, store, private) = setup();
    let artifact = dir.path().join("build.bin");
    fs::write(&artifact, b"build").unwrap();
    store.claim(request()).unwrap();
    assert!(store
        .prepare(
            &artifact,
            "a".repeat(40),
            "cargo test".into(),
            "FAIL".into(),
            "bad".into(),
            vec![]
        )
        .is_err());
    let payload = prepare(&store, &artifact, "good");
    let wrong_key = PKey::generate_ed25519().unwrap();
    let wrong: [u8; 32] = wrong_key.raw_private_key().unwrap().try_into().unwrap();
    assert!(store
        .operator_sign(&artifact, payload.clone(), &wrong)
        .is_err());
    let signed = sign(&store, &artifact, payload, &private);
    assert_eq!(store.handoff(&artifact, signed).unwrap(), "0.1");
}

#[test]
fn a_completed_artifact_must_stay_available_and_unchanged() {
    let (dir, store, private) = setup();
    let artifact = dir.path().join("build.bin");
    fs::write(&artifact, b"immutable artifact").unwrap();
    store.claim(request()).unwrap();
    let signed = sign(
        &store,
        &artifact,
        prepare(&store, &artifact, "r1"),
        &private,
    );
    store.handoff(&artifact, signed).unwrap();
    let stored = fs::read_dir(dir.path().join("state/artifacts"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(stored, b"altered").unwrap();
    assert!(store.next().is_err());
    assert!(store.status().is_err());
}

#[test]
fn a_receipt_for_another_store_is_rejected() {
    let (dir, store, private) = setup();
    let artifact = dir.path().join("build.bin");
    fs::write(&artifact, b"build").unwrap();
    store.claim(request()).unwrap();
    let signed = sign(
        &store,
        &artifact,
        prepare(&store, &artifact, "receipt-1"),
        &private,
    );
    let other = SwarmStore::new(dir.path().join("other"));
    let key = PKey::private_key_from_raw_bytes(&private, openssl::pkey::Id::ED25519).unwrap();
    other
        .initialize(
            plan(),
            &ocaf_g01_swarm::encode_hex(&key.raw_public_key().unwrap()),
        )
        .unwrap();
    other.claim(request()).unwrap();
    assert!(other.handoff(&artifact, signed).is_err());
    assert_eq!(other.status().unwrap()[0].status, StageStatus::Claimed);
}

#[test]
fn concurrent_claims_have_one_winner() {
    let (_dir, store, _private) = setup();
    let store = Arc::new(store);
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let store = store.clone();
            std::thread::spawn(move || store.claim(request()))
        })
        .collect();
    let wins = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .filter(Result::is_ok)
        .count();
    assert_eq!(wins, 1);
    assert_eq!(
        store
            .status()
            .unwrap()
            .iter()
            .filter(|s| s.status == StageStatus::Claimed)
            .count(),
        1
    );
}
