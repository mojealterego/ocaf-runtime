use ocaf_runtime::trs_dtn::{Bundle, DtnError, DtnStore};

fn bundle(id: [u8; 32], expires_at: u64, hops: u8) -> Bundle {
    Bundle { id, ciphertext: vec![0xA5; 8], expires_at, hops_remaining: hops }
}

#[test]
fn forwards_once_per_peer_and_decrements_hops() {
    let mut store = DtnStore::new(4, 64).unwrap();
    store.insert(bundle([1; 32], 100, 3), 10).unwrap();
    let outgoing = store.forward_for("peer-a", 10, 4).unwrap();
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].hops_remaining, 2);
    assert!(store.forward_for("peer-a", 10, 4).unwrap().is_empty());
    assert_eq!(store.forward_for("peer-b", 10, 4).unwrap().len(), 1);
}

#[test]
fn duplicate_and_expired_bundles_fail_closed() {
    let mut store = DtnStore::new(4, 64).unwrap();
    store.insert(bundle([1; 32], 100, 3), 10).unwrap();
    assert_eq!(store.insert(bundle([1; 32], 100, 3), 10), Err(DtnError::Duplicate));
    assert_eq!(store.insert(bundle([2; 32], 9, 3), 10), Err(DtnError::Expired));
    assert_eq!(store.insert(bundle([3; 32], 100, 0), 10), Err(DtnError::InvalidHops));
    assert_eq!(store.purge_expired(101), 1);
    assert_eq!(store.len(), 0);
}

#[test]
fn bounded_capacity_and_payload_size() {
    let mut store = DtnStore::new(1, 8).unwrap();
    store.insert(bundle([1; 32], 100, 1), 10).unwrap();
    assert_eq!(store.insert(bundle([2; 32], 100, 1), 10), Err(DtnError::Capacity));
    assert_eq!(store.insert(Bundle { id: [3; 32], ciphertext: vec![0; 9], expires_at: 100, hops_remaining: 1 }, 10), Err(DtnError::InvalidPayload));
    assert!(store.forward_for("peer", 10, 8).unwrap().is_empty());
}

#[test]
fn invalid_peer_and_clock_expiry_are_rejected() {
    let mut store = DtnStore::new(2, 8).unwrap();
    store.insert(bundle([1; 32], 100, 2), 10).unwrap();
    assert_eq!(store.forward_for("", 10, 1), Err(DtnError::InvalidPeer));
    assert_eq!(store.forward_for("peer", 101, 1), Ok(Vec::new()));
}
