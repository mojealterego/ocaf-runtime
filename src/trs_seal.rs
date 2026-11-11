//! TRS-SEAL v1: authenticated, replay-resistant *symmetric* envelope.
//! This is conventional HMAC-SHA256, not the unproven topological
//! cryptosystem proposed in the TRS research manuscript.
//! Epoch authority and persistent replay storage must be provided by caller.
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::{BTreeMap, BTreeSet};

type HmacSha256 = Hmac<Sha256>;
const DOMAIN: &[u8] = b"OCAF/TRS-SEAL/HMAC-SHA256/v1\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub tenant: String,
    pub sender: String,
    pub key_epoch: u64,
    pub fencing_epoch: u64,
    pub sequence: u64,
    pub payload: Vec<u8>,
    pub tag: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealError {
    InvalidIdentity,
    InvalidKey,
    EpochRollback,
    UnauthorizedFencingEpoch,
    UnknownKeyEpoch,
    InvalidTag,
    Replay,
    SequenceExhausted,
    CapacityExceeded,
}

fn frame(tenant: &str, sender: &str, key_epoch: u64, fencing_epoch: u64, sequence: u64, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(DOMAIN.len() + tenant.len() + sender.len() + payload.len() + 64);
    buf.extend_from_slice(DOMAIN);
    for field in [tenant.as_bytes(), sender.as_bytes(), payload] {
        buf.extend_from_slice(&(field.len() as u64).to_be_bytes());
        buf.extend_from_slice(field);
    }
    for value in [key_epoch, fencing_epoch, sequence] {
        buf.extend_from_slice(&value.to_be_bytes());
    }
    buf
}

fn authenticate(key: &[u8], envelope: &Envelope) -> Result<[u8; 32], SealError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| SealError::InvalidKey)?;
    mac.update(&frame(&envelope.tenant, &envelope.sender, envelope.key_epoch, envelope.fencing_epoch, envelope.sequence, &envelope.payload));
    let bytes = mac.finalize().into_bytes();
    let mut tag = [0u8; 32];
    tag.copy_from_slice(&bytes);
    Ok(tag)
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

/// A key epoch can be rotated only by an authenticated external authority.
/// The prior epoch is optionally retained for verification, never signing.
pub struct SealKeys {
    current_epoch: u64,
    keys: BTreeMap<u64, Vec<u8>>,
}
impl SealKeys {
    pub fn new(epoch: u64, key: &[u8]) -> Result<Self, SealError> {
        if key.len() < 32 { return Err(SealError::InvalidKey); }
        let mut keys = BTreeMap::new();
        keys.insert(epoch, key.to_vec());
        Ok(Self { current_epoch: epoch, keys })
    }
    pub fn rotate(&mut self, new_epoch: u64, new_key: &[u8], retain_previous: bool) -> Result<(), SealError> {
        if new_epoch <= self.current_epoch { return Err(SealError::EpochRollback); }
        if new_key.len() < 32 { return Err(SealError::InvalidKey); }
        let old = self.current_epoch;
        if !retain_previous { self.clear_keys(); }
        else {
            let old_key = self.keys.remove(&old).ok_or(SealError::UnknownKeyEpoch)?;
            self.clear_keys();
            self.keys.insert(old, old_key);
        }
        self.keys.insert(new_epoch, new_key.to_vec());
        self.current_epoch = new_epoch;
        Ok(())
    }
    fn clear_keys(&mut self) {
        for key in self.keys.values_mut() { key.fill(0); }
        self.keys.clear();
    }
    pub fn sign(&self, tenant: &str, sender: &str, fencing_epoch: u64, sequence: u64, payload: &[u8]) -> Result<Envelope, SealError> {
        if !valid_identity(tenant) || !valid_identity(sender) { return Err(SealError::InvalidIdentity); }
        let mut envelope = Envelope {
            tenant: tenant.to_owned(), sender: sender.to_owned(),
            key_epoch: self.current_epoch, fencing_epoch, sequence,
            payload: payload.to_vec(), tag: [0; 32],
        };
        let key = self.keys.get(&self.current_epoch).ok_or(SealError::UnknownKeyEpoch)?;
        envelope.tag = authenticate(key, &envelope)?;
        Ok(envelope)
    }
}
impl Drop for SealKeys {
    fn drop(&mut self) { self.clear_keys(); }
}

/// Replay cache is intentionally bounded and in-memory. Persist or replicate
/// (tenant,sender,key_epoch,fencing_epoch,sequence) before accepting effects.
/// A restart without durable replay state MUST NOT resume the same authority.
pub struct SealVerifier {
    pub authorized_fencing_epoch: u64,
    seen: BTreeSet<(String, String, u64, u64, u64)>,
    capacity: usize,
}
impl SealVerifier {
    pub fn new(authorized_fencing_epoch: u64, capacity: usize) -> Result<Self, SealError> {
        if capacity == 0 { return Err(SealError::CapacityExceeded); }
        Ok(Self { authorized_fencing_epoch, seen: BTreeSet::new(), capacity })
    }
    pub fn verify(&mut self, keys: &SealKeys, envelope: &Envelope) -> Result<(), SealError> {
        if !valid_identity(&envelope.tenant) || !valid_identity(&envelope.sender) {
            return Err(SealError::InvalidIdentity);
        }
        if envelope.fencing_epoch != self.authorized_fencing_epoch {
            return Err(SealError::UnauthorizedFencingEpoch);
        }
        let key = keys.keys.get(&envelope.key_epoch).ok_or(SealError::UnknownKeyEpoch)?;
        let mut mac = HmacSha256::new_from_slice(key).map_err(|_| SealError::InvalidKey)?;
        mac.update(&frame(&envelope.tenant, &envelope.sender, envelope.key_epoch, envelope.fencing_epoch, envelope.sequence, &envelope.payload));
        mac.verify_slice(&envelope.tag).map_err(|_| SealError::InvalidTag)?;
        let identity = (envelope.tenant.clone(), envelope.sender.clone(), envelope.key_epoch, envelope.fencing_epoch, envelope.sequence);
        if self.seen.contains(&identity) { return Err(SealError::Replay); }
        if self.seen.len() >= self.capacity { return Err(SealError::CapacityExceeded); }
        self.seen.insert(identity);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const KEY_A: [u8; 32] = [0x31; 32];
    const KEY_B: [u8; 32] = [0x42; 32];

    #[test]
    fn accepts_once_then_rejects_replay() {
        let keys = SealKeys::new(1, &KEY_A).unwrap();
        let mut verifier = SealVerifier::new(7, 2).unwrap();
        let msg = keys.sign("tenant-a", "node-1", 7, 1, b"hello").unwrap();
        assert_eq!(verifier.verify(&keys, &msg), Ok(()));
        assert_eq!(verifier.verify(&keys, &msg), Err(SealError::Replay));
    }
    #[test]
    fn detects_tamper_and_cross_tenant_rebinding() {
        let keys = SealKeys::new(1, &KEY_A).unwrap();
        let mut verifier = SealVerifier::new(7, 2).unwrap();
        let mut msg = keys.sign("tenant-a", "node-1", 7, 1, b"hello").unwrap();
        msg.tenant = "tenant-b".into();
        assert_eq!(verifier.verify(&keys, &msg), Err(SealError::InvalidTag));
        msg.tenant = "tenant-a".into();
        msg.payload.push(0);
        assert_eq!(verifier.verify(&keys, &msg), Err(SealError::InvalidTag));
    }
    #[test]
    fn rotation_has_bounded_overlap_and_no_old_signing() {
        let mut keys = SealKeys::new(1, &KEY_A).unwrap();
        let old = keys.sign("tenant-a", "node-1", 7, 1, b"old").unwrap();
        keys.rotate(2, &KEY_B, true).unwrap();
        let new = keys.sign("tenant-a", "node-1", 7, 1, b"new").unwrap();
        assert_eq!(new.key_epoch, 2);
        let mut verifier = SealVerifier::new(7, 4).unwrap();
        assert_eq!(verifier.verify(&keys, &old), Ok(()));
        assert_eq!(verifier.verify(&keys, &new), Ok(()));
        keys.rotate(3, &KEY_A, false).unwrap();
        assert_eq!(verifier.verify(&keys, &old), Err(SealError::UnknownKeyEpoch));
        assert_eq!(keys.rotate(3, &KEY_B, false), Err(SealError::EpochRollback));
    }
    #[test]
    fn rejects_untrusted_fencing_and_capacity_exhaustion() {
        let keys = SealKeys::new(1, &KEY_A).unwrap();
        let mut verifier = SealVerifier::new(7, 1).unwrap();
        let stale = keys.sign("tenant-a", "node-1", 6, 1, b"x").unwrap();
        assert_eq!(verifier.verify(&keys, &stale), Err(SealError::UnauthorizedFencingEpoch));
        let first = keys.sign("tenant-a", "node-1", 7, 1, b"x").unwrap();
        let second = keys.sign("tenant-a", "node-1", 7, 2, b"x").unwrap();
        assert_eq!(verifier.verify(&keys, &first), Ok(()));
        assert_eq!(verifier.verify(&keys, &second), Err(SealError::CapacityExceeded));
    }
    #[test]
    fn rejected_rotation_preserves_current_signing_key() {
        let mut keys = SealKeys::new(7, &KEY_A).unwrap();
        assert_eq!(keys.rotate(8, &[1u8; 31], false), Err(SealError::InvalidKey));
        let msg = keys.sign("tenant-a", "node-1", 4, 9, b"after-error").unwrap();
        assert_eq!(msg.key_epoch, 7);
        let mut verifier = SealVerifier::new(4, 1).unwrap();
        assert_eq!(verifier.verify(&keys, &msg), Ok(()));
    }

    #[test]
    fn different_sender_cannot_reuse_authenticated_tag() {
        let keys = SealKeys::new(1, &KEY_A).unwrap();
        let mut verifier = SealVerifier::new(4, 2).unwrap();
        let mut msg = keys.sign("tenant-a", "sender-a", 4, 1, b"data").unwrap();
        msg.sender = "sender-b".to_string();
        assert_eq!(verifier.verify(&keys, &msg), Err(SealError::InvalidTag));
    }

}
