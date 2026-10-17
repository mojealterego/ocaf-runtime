//! Bounds-checked typed frames for a future Zenoh shared-memory provider.
//!
//! This module deliberately does not claim to allocate or map shared memory.
//! It provides the memory-safe wire contract that a provider must satisfy
//! before handing a payload to the runtime: versioned header, bounded length,
//! and an integrity digest over the exact bytes.

use sha2::{Digest, Sha256};
use thiserror::Error;

const MAGIC: &[u8; 4] = b"OCSH";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 4 + 1 + 4 + 32;
pub const MAX_PAYLOAD_BYTES: usize = 1 << 20;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ShmFrameError {
    #[error("frame is truncated")]
    Truncated,
    #[error("unsupported frame header")]
    Header,
    #[error("payload exceeds the configured bound")]
    Oversized,
    #[error("payload digest mismatch")]
    Integrity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShmFrame {
    payload: Vec<u8>,
}

impl ShmFrame {
    pub fn new(payload: impl Into<Vec<u8>>) -> Result<Self, ShmFrameError> {
        let payload = payload.into();
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(ShmFrameError::Oversized);
        }
        Ok(Self { payload })
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + self.payload.len());
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        out.extend_from_slice(&digest(&self.payload));
        out.extend_from_slice(&self.payload);
        out
    }

    pub fn decode(frame: &[u8]) -> Result<Self, ShmFrameError> {
        if frame.len() < HEADER_LEN {
            return Err(ShmFrameError::Truncated);
        }
        if &frame[..4] != MAGIC || frame[4] != VERSION {
            return Err(ShmFrameError::Header);
        }
        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&frame[5..9]);
        let length = u32::from_be_bytes(len_bytes) as usize;
        if length > MAX_PAYLOAD_BYTES {
            return Err(ShmFrameError::Oversized);
        }
        if frame.len() != HEADER_LEN + length {
            return Err(ShmFrameError::Truncated);
        }
        let expected = &frame[9..HEADER_LEN];
        let payload = frame[HEADER_LEN..].to_vec();
        if expected != digest(&payload).as_slice() {
            return Err(ShmFrameError::Integrity);
        }
        Ok(Self { payload })
    }
}

fn digest(payload: &[u8]) -> [u8; 32] {
    Sha256::digest(payload).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_payload() {
        let frame = ShmFrame::new(b"zenoh-local".to_vec()).unwrap();
        assert_eq!(ShmFrame::decode(&frame.encode()).unwrap(), frame);
    }

    #[test]
    fn rejects_tampering() {
        let frame = ShmFrame::new(b"payload".to_vec()).unwrap();
        let mut encoded = frame.encode();
        *encoded.last_mut().unwrap() ^= 1;
        assert_eq!(ShmFrame::decode(&encoded), Err(ShmFrameError::Integrity));
    }

    #[test]
    fn rejects_oversized_payload_before_encoding() {
        assert_eq!(
            ShmFrame::new(vec![0u8; MAX_PAYLOAD_BYTES + 1]),
            Err(ShmFrameError::Oversized)
        );
    }

    #[test]
    fn rejects_trailing_bytes() {
        let frame = ShmFrame::new(vec![1, 2, 3]).unwrap();
        let mut encoded = frame.encode();
        encoded.push(4);
        assert_eq!(ShmFrame::decode(&encoded), Err(ShmFrameError::Truncated));
    }
}
