//! Bounded, transport-independent cover-traffic scheduler.
//!
//! Cover packets only reduce one class of timing leakage. They do not provide
//! anonymity, indistinguishability from ciphertext, or a constant bit rate.
//! The caller must encrypt/authenticate every emitted packet using its
//! transport protocol. Never transmit raw output of `next_packet`.
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverError {
    InvalidConfig,
    InvalidPacketSize,
    QueueFull,
    EntropyFailure,
}

pub struct CoverScheduler {
    packet_bytes: usize,
    rate_per_second: f64,
    capacity: usize,
    pending: VecDeque<Vec<u8>>,
}

impl CoverScheduler {
    /// Rate is Poisson arrivals per second; this is NOT a constant-rate stream.
    /// Limits bound memory use and maximum generated packet size.
    pub fn new(packet_bytes: usize, rate_per_second: f64, capacity: usize) -> Result<Self, CoverError> {
        if packet_bytes == 0 || packet_bytes > 65_536 || capacity == 0 ||
            capacity > 4096 || !rate_per_second.is_finite() ||
            rate_per_second <= 0.0 || rate_per_second > 1000.0 {
            return Err(CoverError::InvalidConfig);
        }
        Ok(Self { packet_bytes, rate_per_second, capacity, pending: VecDeque::new() })
    }

    pub fn enqueue(&mut self, ciphertext: Vec<u8>) -> Result<(), CoverError> {
        if ciphertext.len() != self.packet_bytes {
            return Err(CoverError::InvalidPacketSize);
        }
        if self.pending.len() >= self.capacity {
            return Err(CoverError::QueueFull);
        }
        self.pending.push_back(ciphertext);
        Ok(())
    }

    pub fn queued(&self) -> usize { self.pending.len() }

    /// Cryptographically random exponential interarrival duration.
    /// Saturates at one hour so a malformed or extreme random sample cannot
    /// stall an entire session forever.
    pub fn next_delay(&self) -> Result<Duration, CoverError> {
        let mut random = [0u8; 8];
        openssl::rand::rand_bytes(&mut random).map_err(|_| CoverError::EntropyFailure)?;
        let sample = u64::from_be_bytes(random);
        // u in (0,1), avoiding ln(0) and ln(1) endpoints.
        let u = ((sample as f64) + 0.5) / ((u64::MAX as f64) + 1.0);
        let u = u.clamp(f64::MIN_POSITIVE, 1.0 - f64::EPSILON);
        let seconds = (-(-u).ln_1p() / self.rate_per_second).min(3600.0);
        Ok(Duration::from_secs_f64(seconds))
    }

    /// Return one fixed-length frame. `is_cover` is local metadata and MUST
    /// NOT be transmitted as an unencrypted distinguishing bit.
    /// The transport must wrap both cover and real frames in the same
    /// authenticated encryption format and reject cover frames after decrypting.
    pub fn next_packet(&mut self) -> Result<(Vec<u8>, bool), CoverError> {
        if let Some(real) = self.pending.pop_front() {
            return Ok((real, false));
        }
        let mut cover = vec![0u8; self.packet_bytes];
        openssl::rand::rand_bytes(&mut cover).map_err(|_| CoverError::EntropyFailure)?;
        Ok((cover, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_bounded_queue_and_fixed_frame_size() {
        let mut s = CoverScheduler::new(32, 2.0, 1).unwrap();
        assert_eq!(s.enqueue(vec![1; 31]), Err(CoverError::InvalidPacketSize));
        assert_eq!(s.enqueue(vec![1; 32]), Ok(()));
        assert_eq!(s.enqueue(vec![2; 32]), Err(CoverError::QueueFull));
        assert_eq!(s.queued(), 1);
        assert_eq!(s.next_packet().unwrap(), (vec![1; 32], false));
        assert_eq!(s.queued(), 0);
    }

    #[test]
    fn cover_is_generated_at_exact_length() {
        let mut s = CoverScheduler::new(256, 3.0, 2).unwrap();
        let (cover, flag) = s.next_packet().unwrap();
        assert!(flag);
        assert_eq!(cover.len(), 256);
    }

    #[test]
    fn invalid_configuration_fails_closed() {
        assert!(matches!(CoverScheduler::new(0, 2.0, 1), Err(CoverError::InvalidConfig)));
        assert!(matches!(CoverScheduler::new(32, f64::NAN, 1), Err(CoverError::InvalidConfig)));
        assert!(matches!(CoverScheduler::new(32, 2.0, 0), Err(CoverError::InvalidConfig)));
    }

    #[test]
    fn random_delays_are_finite_and_bounded() {
        let s = CoverScheduler::new(32, 2.0, 1).unwrap();
        for _ in 0..100 {
            assert!(s.next_delay().unwrap() <= Duration::from_secs(3600));
        }
    }
}
