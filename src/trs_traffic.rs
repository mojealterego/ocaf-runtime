//! Bounded fixed-size traffic shaping primitives for experimental transports.
//!
//! This module DOES NOT provide steganography, sender anonymity, packet
//! authentication or protection from global traffic correlation. The caller
//! must encrypt and authenticate real packets and transport chaff in a format
//! indistinguishable to its actual adversary model.
use std::collections::VecDeque;
use openssl::rand::rand_bytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrafficConfig {
    pub packet_size: usize,
    pub capacity: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficError {
    InvalidConfig,
    InvalidPacketSize,
    QueueFull,
    InvalidRate,
    RandomFailure,
}

pub struct TrafficQueue {
    config: TrafficConfig,
    pending: VecDeque<Vec<u8>>,
}

impl TrafficQueue {
    /// Enforces a bounded queue and a bounded packet allocation.
    pub fn new(config: TrafficConfig) -> Result<Self, TrafficError> {
        if config.packet_size == 0 || config.packet_size > 65_536
            || config.capacity == 0 || config.capacity > 4_096
            || config.packet_size.checked_mul(config.capacity).map_or(true, |n| n > 64 * 1024 * 1024)
        {
            return Err(TrafficError::InvalidConfig);
        }
        Ok(Self { config, pending: VecDeque::new() })
    }

    pub fn len(&self) -> usize { self.pending.len() }
    pub fn is_empty(&self) -> bool { self.pending.is_empty() }

    /// No silent drop of authenticated user data: backpressure is explicit.
    pub fn enqueue(&mut self, packet: Vec<u8>) -> Result<(), TrafficError> {
        if packet.len() != self.config.packet_size {
            return Err(TrafficError::InvalidPacketSize);
        }
        if self.pending.len() >= self.config.capacity {
            return Err(TrafficError::QueueFull);
        }
        self.pending.push_back(packet);
        Ok(())
    }

    /// Emits a real queued packet, or random cover bytes of identical size.
    /// Cover bytes alone are NOT a valid encrypted transport packet.
    pub fn next_packet(&mut self) -> Result<Vec<u8>, TrafficError> {
        if let Some(packet) = self.pending.pop_front() {
            return Ok(packet);
        }
        let mut cover = vec![0u8; self.config.packet_size];
        rand_bytes(&mut cover).map_err(|_| TrafficError::RandomFailure)?;
        Ok(cover)
    }
}

/// Exponential inter-arrival delay of a homogeneous Poisson process.
/// Uses a cryptographic random source; delay is strictly positive.
pub fn exponential_delay_seconds(rate: f64) -> Result<f64, TrafficError> {
    if !rate.is_finite() || rate <= 0.0 {
        return Err(TrafficError::InvalidRate);
    }
    let mut bytes = [0u8; 8];
    rand_bytes(&mut bytes).map_err(|_| TrafficError::RandomFailure)?;
    let bits = u64::from_be_bytes(bytes) >> 11; // 53 random bits
    let u = (bits as f64 + 0.5) / ((1u64 << 53) as f64);
    let delay = -(-u).ln_1p() / rate;
    if !delay.is_finite() || delay <= 0.0 {
        return Err(TrafficError::InvalidRate);
    }
    Ok(delay)
}
