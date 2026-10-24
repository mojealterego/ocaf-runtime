//! Fail-closed mTLS peer registry tests for OMAs stage 0.43.
use ocaf_runtime::stage043_mtls_admission::{PeerRegistry, TrustedPeer};
use std::collections::HashMap;

#[test]
fn empty_peer_registry_is_denied() {
    assert!(PeerRegistry::new(HashMap::new()).is_err());
}

#[test]
fn untrusted_sender_syntax_is_denied() {
    let mut peers = HashMap::new();
    peers.insert(
        [1u8; 32],
        TrustedPeer {
            sender: "attacker/forged".into(),
            signing_key: [2u8; 32],
        },
    );
    assert!(PeerRegistry::new(peers).is_err());
}

#[test]
fn operator_provisioned_valid_peer_is_accepted() {
    let mut peers = HashMap::new();
    peers.insert(
        [1u8; 32],
        TrustedPeer {
            sender: "node_001".into(),
            signing_key: [2u8; 32],
        },
    );
    assert!(PeerRegistry::new(peers).is_ok());
}
