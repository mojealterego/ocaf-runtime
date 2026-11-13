//! Reliable DTN delivery bookkeeping: bounded retries and explicit acknowledgements.
//! Experimental state only. Caller must authenticate peers, use trusted time and
//! encrypt/authenticate bundles independently. Not a complete mesh transport.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReliableBundle {
    pub id: [u8; 32],
    pub ciphertext: Vec<u8>,
    pub expires_at: u64,
    pub hops_remaining: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReliableError { InvalidConfig, InvalidBundle, InvalidPeer, Duplicate, Capacity, Expired, NotInFlight, UnknownBundle }
struct Entry {
    bundle: ReliableBundle,
    pending: BTreeMap<String, u64>,
    acknowledged: BTreeSet<String>,
}
pub struct ReliableDtn {
    max_bundles: usize,
    max_payload: usize,
    retry_secs: u64,
    entries: BTreeMap<[u8; 32], Entry>,
}
fn peer_valid(peer: &str) -> bool {
    !peer.is_empty() && peer.len() <= 256 && !peer.chars().any(char::is_control)
}
impl ReliableDtn {
    pub fn new(max_bundles: usize, max_payload: usize, retry_secs: u64) -> Result<Self, ReliableError> {
        if max_bundles == 0 || max_bundles > 4096 || max_payload == 0 || max_payload > 65536
            || retry_secs == 0 || max_bundles.checked_mul(max_payload).map_or(true, |n| n > 64 * 1024 * 1024) {
            return Err(ReliableError::InvalidConfig);
        }
        Ok(Self { max_bundles, max_payload, retry_secs, entries: BTreeMap::new() })
    }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn insert(&mut self, bundle: ReliableBundle, now: u64) -> Result<(), ReliableError> {
        if bundle.ciphertext.is_empty() || bundle.ciphertext.len() > self.max_payload || bundle.hops_remaining == 0 {
            return Err(ReliableError::InvalidBundle);
        }
        if bundle.expires_at <= now { return Err(ReliableError::Expired); }
        if self.entries.contains_key(&bundle.id) { return Err(ReliableError::Duplicate); }
        if self.entries.len() >= self.max_bundles { return Err(ReliableError::Capacity); }
        self.entries.insert(bundle.id, Entry { bundle, pending: BTreeMap::new(), acknowledged: BTreeSet::new() });
        Ok(())
    }
    pub fn purge_expired(&mut self, now: u64) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, entry| entry.bundle.expires_at > now);
        before - self.entries.len()
    }
    pub fn select(&mut self, peer: &str, now: u64, limit: usize) -> Result<Vec<ReliableBundle>, ReliableError> {
        if !peer_valid(peer) { return Err(ReliableError::InvalidPeer); }
        if limit == 0 || limit > 4096 { return Err(ReliableError::Capacity); }
        self.purge_expired(now);
        let mut out = Vec::new();
        for entry in self.entries.values_mut() {
            if out.len() == limit { break; }
            if entry.bundle.hops_remaining <= 1 || entry.acknowledged.contains(peer) { continue; }
            if let Some(&deadline) = entry.pending.get(peer) {
                if now < deadline { continue; }
            } else if entry.pending.len() + entry.acknowledged.len() >= 64 { continue; }
            let mut copy = entry.bundle.clone();
            copy.hops_remaining -= 1;
            entry.pending.insert(peer.to_owned(), now.saturating_add(self.retry_secs).min(entry.bundle.expires_at));
            out.push(copy);
        }
        Ok(out)
    }
    /// Release an in-flight lease after a locally detected transport failure.
    /// This does not acknowledge delivery and allows immediate retransmission.
    /// Only call after the transport reports failure; an ambiguous timeout
    /// can result in duplicate delivery, which receivers must deduplicate.
    pub fn cancel_pending(&mut self, id: &[u8; 32], peer: &str) -> Result<(), ReliableError> {
        if !peer_valid(peer) { return Err(ReliableError::InvalidPeer); }
        let entry = self.entries.get_mut(id).ok_or(ReliableError::UnknownBundle)?;
        if entry.pending.remove(peer).is_none() {
            return Err(ReliableError::NotInFlight);
        }
        Ok(())
    }
    pub fn acknowledge(&mut self, id: &[u8; 32], peer: &str, now: u64) -> Result<(), ReliableError> {
        if !peer_valid(peer) { return Err(ReliableError::InvalidPeer); }
        let entry = self.entries.get_mut(id).ok_or(ReliableError::UnknownBundle)?;
        if entry.bundle.expires_at <= now { return Err(ReliableError::Expired); }
        match entry.pending.get(peer) {
            Some(&deadline) if now <= deadline => {},
            _ => return Err(ReliableError::NotInFlight),
        }
        entry.pending.remove(peer);
        entry.acknowledged.insert(peer.to_owned());
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn bundle(id: u8, hops: u8) -> ReliableBundle {
        ReliableBundle { id: [id; 32], ciphertext: vec![1; 16], expires_at: 100, hops_remaining: hops }
    }
    #[test]
    fn retry_then_ack() {
        let mut dtn = ReliableDtn::new(2, 32, 5).unwrap();
        dtn.insert(bundle(1, 3), 0).unwrap();
        assert_eq!(dtn.select("peer", 10, 1).unwrap()[0].hops_remaining, 2);
        assert!(dtn.select("peer", 14, 1).unwrap().is_empty());
        assert_eq!(dtn.select("peer", 15, 1).unwrap().len(), 1);
        dtn.acknowledge(&[1; 32], "peer", 15).unwrap();
        assert!(dtn.select("peer", 20, 1).unwrap().is_empty());
    }
    #[test]
    fn expired_retry_lease_cannot_be_acknowledged() {
        let mut dtn = ReliableDtn::new(2, 32, 5).unwrap();
        dtn.insert(bundle(3, 3), 0).unwrap();
        assert_eq!(dtn.select("peer", 10, 1).unwrap().len(), 1);
        assert_eq!(dtn.acknowledge(&[3; 32], "peer", 16), Err(ReliableError::NotInFlight));
        assert_eq!(dtn.select("peer", 16, 1).unwrap().len(), 1);
        assert_eq!(dtn.acknowledge(&[3; 32], "peer", 17), Ok(()));
    }
    #[test]
    fn retry_deadline_is_clamped_to_bundle_expiration() {
        let mut dtn = ReliableDtn::new(2, 32, u64::MAX).unwrap();
        dtn.insert(bundle(4, 3), 0).unwrap();
        assert_eq!(dtn.select("peer", 99, 1).unwrap().len(), 1);
        assert_eq!(dtn.acknowledge(&[4; 32], "peer", 100), Err(ReliableError::Expired));
    }
    #[test]
    fn select_rejects_unbounded_batch_without_mutating_state() {
        let mut dtn = ReliableDtn::new(1, 32, 5).unwrap();
        dtn.insert(bundle(9, 3), 0).unwrap();
        assert_eq!(dtn.select("peer", 10, usize::MAX), Err(ReliableError::Capacity));
        assert_eq!(dtn.select("peer", 10, 1).unwrap().len(), 1);
    }
    #[test]
    fn stale_ack_does_not_release_pending_fanout_slot() {
        let mut dtn = ReliableDtn::new(1, 32, 5).unwrap();
        dtn.insert(bundle(8, 3), 0).unwrap();
        assert_eq!(dtn.select("peer", 10, 1).unwrap().len(), 1);
        assert_eq!(dtn.acknowledge(&[8;32], "peer", 16), Err(ReliableError::NotInFlight));
        assert_eq!(dtn.select("peer", 16, 1).unwrap().len(), 1);
    }
    #[test]
    fn validation_and_expiry() {
        let mut dtn = ReliableDtn::new(1, 32, 5).unwrap();
        assert_eq!(dtn.insert(bundle(1, 0), 0), Err(ReliableError::InvalidBundle));
        dtn.insert(bundle(1, 2), 0).unwrap();
        assert_eq!(dtn.insert(bundle(1, 2), 0), Err(ReliableError::Duplicate));
        assert_eq!(dtn.insert(bundle(2, 2), 0), Err(ReliableError::Capacity));
        assert_eq!(dtn.acknowledge(&[1; 32], "peer", 0), Err(ReliableError::NotInFlight));
        assert_eq!(dtn.purge_expired(100), 1);
    }
}
