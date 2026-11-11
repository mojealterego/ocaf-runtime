//! Experimental delay-tolerant store-and-forward for opaque encrypted bundles.
//! Not a radio transport, not an anonymity layer, not an authenticated channel.
//! Callers MUST authenticate/decrypt bundles at endpoints and authorize peers.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub id: [u8; 32],
    pub ciphertext: Vec<u8>,
    pub expires_at: u64,
    pub hops_remaining: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DtnError {
    InvalidConfig,
    InvalidPayload,
    InvalidHops,
    InvalidPeer,
    Expired,
    Duplicate,
    Capacity,
}

struct Stored {
    bundle: Bundle,
    forwarded_to: BTreeSet<String>,
}

pub struct DtnStore {
    max_bundles: usize,
    max_payload: usize,
    bundles: BTreeMap<[u8; 32], Stored>,
}

impl DtnStore {
    /// Limits are per-process, not durable. No unbounded queues or TTL extension.
    pub fn new(max_bundles: usize, max_payload: usize) -> Result<Self, DtnError> {
        if max_bundles == 0 || max_bundles > 4096 ||
            max_payload == 0 || max_payload > 65_536 ||
            max_bundles.checked_mul(max_payload).map_or(true, |n| n > 64 * 1024 * 1024) {
            return Err(DtnError::InvalidConfig);
        }
        Ok(Self { max_bundles, max_payload, bundles: BTreeMap::new() })
    }

    pub fn len(&self) -> usize { self.bundles.len() }
    pub fn is_empty(&self) -> bool { self.bundles.is_empty() }

    pub fn insert(&mut self, bundle: Bundle, now: u64) -> Result<(), DtnError> {
        if bundle.ciphertext.is_empty() || bundle.ciphertext.len() > self.max_payload {
            return Err(DtnError::InvalidPayload);
        }
        if bundle.hops_remaining == 0 { return Err(DtnError::InvalidHops); }
        if bundle.expires_at <= now { return Err(DtnError::Expired); }
        if self.bundles.contains_key(&bundle.id) { return Err(DtnError::Duplicate); }
        if self.bundles.len() >= self.max_bundles { return Err(DtnError::Capacity); }
        self.bundles.insert(bundle.id, Stored { bundle, forwarded_to: BTreeSet::new() });
        Ok(())
    }

    /// Removes bundles that are expired according to a trusted monotonic time source.
    pub fn purge_expired(&mut self, now: u64) -> usize {
        let before = self.bundles.len();
        self.bundles.retain(|_, item| item.bundle.expires_at > now);
        before - self.bundles.len()
    }

    /// Returns a bounded batch of forwarded copies with one hop consumed.
    /// Repeated calls for the same peer do not resend the same bundle.
    /// The caller is responsible for transport retries/acknowledgements.
    pub fn forward_for(
        &mut self,
        peer: &str,
        now: u64,
        max_items: usize,
    ) -> Result<Vec<Bundle>, DtnError> {
        if peer.is_empty() || peer.len() > 256 || peer.chars().any(char::is_control) {
            return Err(DtnError::InvalidPeer);
        }
        if max_items == 0 {
            return Err(DtnError::Capacity);
        }
        self.purge_expired(now);
        let mut batch = Vec::new();
        for item in self.bundles.values_mut() {
            if batch.len() == max_items { break; }
            if item.bundle.hops_remaining <= 1 || item.forwarded_to.contains(peer) || item.forwarded_to.len() >= 64 {
                continue;
            }
            let mut outgoing = item.bundle.clone();
            outgoing.hops_remaining -= 1;
            item.forwarded_to.insert(peer.to_owned());
            batch.push(outgoing);
        }
        Ok(batch)
    }
}
