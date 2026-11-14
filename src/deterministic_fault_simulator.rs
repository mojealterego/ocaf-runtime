//! Deterministic, bounded virtual-time fault simulator for OCAF Runtime.
//!
//! Reference: "Architektura i Kod OCAF Runtime", pages 39-40, deterministic
//! simulator (LCG + scheduled events). This hardens that *reference design*:
//! - future events stay queued across segmented runs;
//! - virtual time, IDs, memory, queue length, unique links and steps are bounded;
//! - partition, drop, duplication and corruption are deterministic and isolated
//!   by tenant; ordering is stable for equal timestamps;
//! - the transcript SHA-256 is an UNSIGNED simulation fingerprint, not a
//!   hardware receipt, network attestation or production approval.
//!
//! The simulator itself has no OS clock, threads, network or filesystem I/O.
//! The caller-supplied delivery callback is NOT sandboxed or side-effect-free
//! by construction; production executors MUST NOT treat this as authorization.

use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap};

const MAX_QUEUE: usize = 4096;
const MAX_BUFFERED_BYTES: usize = 16 * 1024 * 1024;
const MAX_PAYLOAD_BYTES: usize = 64 * 1024;
const MAX_DISTINCT_LINKS: usize = 4096;
const MAX_STEPS: usize = 100_000;
const LCG_FACTOR: u64 = 6_364_136_223_846_793_005;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimMessage {
    pub tenant: String,
    pub sender: String,
    pub recipient: String,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryKind {
    Primary,
    InjectedDuplicate,
    InjectedCorruption,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimError {
    InvalidConfiguration,
    InvalidIdentity,
    InvalidLink,
    InvalidHorizon,
    PayloadLimit,
    QueueLimit,
    LinkLimit,
    ClockOverflow,
    SequenceExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaultPolicy {
    pub drop_percent: u8,
    pub duplicate_percent: u8,
    pub corrupt_percent: u8,
}

impl Default for FaultPolicy {
    fn default() -> Self {
        // Mirrors the reference's 5% deterministic duplicate scenario.
        Self {
            drop_percent: 0,
            duplicate_percent: 5,
            corrupt_percent: 0,
        }
    }
}

impl FaultPolicy {
    fn valid(self) -> bool {
        u16::from(self.drop_percent)
            + u16::from(self.duplicate_percent)
            + u16::from(self.corrupt_percent)
            <= 100
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimulatorLimits {
    pub max_pending_events: usize,
    pub max_queued_bytes: usize,
    pub max_payload_bytes: usize,
    pub max_steps_per_run: usize,
}

impl Default for SimulatorLimits {
    fn default() -> Self {
        Self {
            max_pending_events: 256,
            max_queued_bytes: 1024 * 1024,
            max_payload_bytes: 64 * 1024,
            max_steps_per_run: 10_000,
        }
    }
}

impl SimulatorLimits {
    fn valid(self) -> bool {
        (1..=MAX_QUEUE).contains(&self.max_pending_events)
            && (1..=MAX_BUFFERED_BYTES).contains(&self.max_queued_bytes)
            && (1..=MAX_PAYLOAD_BYTES).contains(&self.max_payload_bytes)
            && (1..=MAX_STEPS).contains(&self.max_steps_per_run)
            && self.max_payload_bytes <= self.max_queued_bytes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SimEvent {
    Message(SimMessage),
    Partition {
        tenant: String,
        low: String,
        high: String,
        enabled: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TimedEvent {
    at_ms: u64,
    id: u64,
    queued_bytes: usize,
    event: SimEvent,
}

impl Ord for TimedEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .at_ms
            .cmp(&self.at_ms)
            .then_with(|| other.id.cmp(&self.id))
    }
}

impl PartialOrd for TimedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':')
        })
}

type LinkKey = (String, String, String);

fn link_key(tenant: &str, a: &str, b: &str) -> Result<LinkKey, SimError> {
    if !valid_id(tenant) || !valid_id(a) || !valid_id(b) {
        return Err(SimError::InvalidIdentity);
    }
    if a == b {
        return Err(SimError::InvalidLink);
    }
    Ok(if a < b {
        (tenant.to_owned(), a.to_owned(), b.to_owned())
    } else {
        (tenant.to_owned(), b.to_owned(), a.to_owned())
    })
}

fn update_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulationReceipt {
    pub processed_events: usize,
    pub delivered_callbacks: usize,
    pub dropped_messages: usize,
    pub duplicate_callbacks: usize,
    pub corrupted_callbacks: usize,
    pub partition_changes: usize,
    pub pending_events: usize,
    pub budget_exhausted: bool,
    pub current_time_ms: u64,
    /// UNSIGNED fingerprint of simulated event decisions. Not a real-world
    /// effect receipt and not an assertion of external infrastructure.
    pub transcript_sha256: [u8; 32],
    simulation_only: bool,
}

impl SimulationReceipt {
    /// Cannot be changed by a consumer to claim a hardware or production run.
    pub fn is_simulation_only(&self) -> bool {
        self.simulation_only
    }
}

#[derive(Clone, Debug)]
pub struct DeterministicSimulator {
    now_ms: u64,
    next_id: u64,
    prng_state: u64,
    limits: SimulatorLimits,
    policy: FaultPolicy,
    events: BinaryHeap<TimedEvent>,
    queued_bytes: usize,
    known_links: BTreeSet<LinkKey>,
    partitions: BTreeSet<LinkKey>,
    transcript_sha256: [u8; 32],
}

impl DeterministicSimulator {
    pub fn new(seed: u64, limits: SimulatorLimits, policy: FaultPolicy) -> Result<Self, SimError> {
        if !limits.valid() || !policy.valid() {
            return Err(SimError::InvalidConfiguration);
        }
        Ok(Self {
            now_ms: 0,
            next_id: 0,
            prng_state: seed,
            limits,
            policy,
            events: BinaryHeap::new(),
            queued_bytes: 0,
            known_links: BTreeSet::new(),
            partitions: BTreeSet::new(),
            transcript_sha256: [0; 32],
        })
    }

    pub fn now_ms(&self) -> u64 {
        self.now_ms
    }

    pub fn pending_events(&self) -> usize {
        self.events.len()
    }

    pub fn queued_bytes(&self) -> usize {
        self.queued_bytes
    }

    pub fn trace_digest(&self) -> [u8; 32] {
        self.transcript_sha256
    }

    pub fn is_partitioned(&self, tenant: &str, a: &str, b: &str) -> Result<bool, SimError> {
        Ok(self.partitions.contains(&link_key(tenant, a, b)?))
    }

    fn enqueue(&mut self, delay_ms: u64, kind: SimEvent, bytes: usize) -> Result<u64, SimError> {
        let at_ms = self.now_ms.checked_add(delay_ms).ok_or(SimError::ClockOverflow)?;
        let new_id = self.next_id.checked_add(1).ok_or(SimError::SequenceExhausted)?;
        let new_bytes = self
            .queued_bytes
            .checked_add(bytes)
            .ok_or(SimError::QueueLimit)?;
        if self.events.len() >= self.limits.max_pending_events
            || new_bytes > self.limits.max_queued_bytes
        {
            return Err(SimError::QueueLimit);
        }
        self.events.push(TimedEvent {
            at_ms,
            id: new_id,
            queued_bytes: bytes,
            event: kind,
        });
        self.next_id = new_id;
        self.queued_bytes = new_bytes;
        Ok(new_id)
    }

    /// Take ownership of the message; the caller cannot mutate the scheduled
    /// event afterward. This is synthetic data, not authenticated ingress.
    pub fn schedule_message(&mut self, delay_ms: u64, message: SimMessage) -> Result<u64, SimError> {
        if !valid_id(&message.tenant)
            || !valid_id(&message.sender)
            || !valid_id(&message.recipient)
        {
            return Err(SimError::InvalidIdentity);
        }
        if message.payload.len() > self.limits.max_payload_bytes {
            return Err(SimError::PayloadLimit);
        }
        let bytes = message.payload.len();
        self.enqueue(delay_ms, SimEvent::Message(message), bytes)
    }

    /// Schedule an explicitly scoped network partition. No other tenant's link
    /// is affected. Link identities are canonicalized independent of direction.
    pub fn schedule_partition(
        &mut self,
        delay_ms: u64,
        tenant: &str,
        a: &str,
        b: &str,
        enabled: bool,
    ) -> Result<u64, SimError> {
        let key = link_key(tenant, a, b)?;
        if !self.known_links.contains(&key) && self.known_links.len() >= MAX_DISTINCT_LINKS {
            return Err(SimError::LinkLimit);
        }
        let id = self.enqueue(
            delay_ms,
            SimEvent::Partition {
                tenant: key.0.clone(),
                low: key.1.clone(),
                high: key.2.clone(),
                enabled,
            },
            0,
        )?;
        self.known_links.insert(key);
        Ok(id)
    }

    fn draw_percent(&mut self) -> u16 {
        self.prng_state = self
            .prng_state
            .wrapping_mul(LCG_FACTOR)
            .wrapping_add(1);
        (self.prng_state % 100) as u16
    }

    fn record(&mut self, event: &TimedEvent, verdict: &[u8], roll: Option<u16>) {
        let mut h = Sha256::new();
        h.update(b"OCAF-DETERMINISTIC-SIM-TRANSCRIPT-v1");
        h.update(self.transcript_sha256);
        h.update(event.at_ms.to_be_bytes());
        h.update(event.id.to_be_bytes());
        update_field(&mut h, verdict);
        h.update(roll.unwrap_or(u16::MAX).to_be_bytes());
        match &event.event {
            SimEvent::Message(msg) => {
                update_field(&mut h, b"MESSAGE");
                for item in [
                    msg.tenant.as_bytes(),
                    msg.sender.as_bytes(),
                    msg.recipient.as_bytes(),
                    msg.payload.as_slice(),
                ] {
                    update_field(&mut h, item);
                }
            }
            SimEvent::Partition {
                tenant,
                low,
                high,
                enabled,
            } => {
                update_field(&mut h, b"PARTITION");
                for value in [tenant.as_bytes(), low.as_bytes(), high.as_bytes()] {
                    update_field(&mut h, value);
                }
                h.update([u8::from(*enabled)]);
            }
        }
        self.transcript_sha256 = h.finalize().into();
    }

    /// Advance deterministically through events whose timestamp <= horizon.
    /// Events beyond the horizon are never popped or discarded.
    ///
    /// The handler may schedule additional events, but cannot gain real-world
    /// permissions from this simulator. If the processing budget is exhausted,
    /// due events remain queued and current time is NOT advanced to horizon.
    /// Duplicate delivery is explicit and must be deduplicated by consumers.
    pub fn run_until<F>(
        &mut self,
        horizon_ms: u64,
        max_steps: usize,
        mut handler: F,
    ) -> Result<SimulationReceipt, SimError>
    where
        F: FnMut(&SimMessage, DeliveryKind, &mut Self),
    {
        if horizon_ms < self.now_ms || max_steps == 0 || max_steps > self.limits.max_steps_per_run {
            return Err(SimError::InvalidHorizon);
        }
        let mut receipt = SimulationReceipt {
            processed_events: 0,
            delivered_callbacks: 0,
            dropped_messages: 0,
            duplicate_callbacks: 0,
            corrupted_callbacks: 0,
            partition_changes: 0,
            pending_events: 0,
            budget_exhausted: false,
            current_time_ms: self.now_ms,
            transcript_sha256: self.transcript_sha256,
            simulation_only: true,
        };
        while receipt.processed_events < max_steps {
            let Some(next) = self.events.peek() else {
                break;
            };
            if next.at_ms > horizon_ms {
                break;
            }
            let event = self.events.pop().expect("queue peek preceded pop");
            self.queued_bytes -= event.queued_bytes;
            self.now_ms = event.at_ms;
            receipt.processed_events += 1;
            match &event.event {
                SimEvent::Partition {
                    tenant,
                    low,
                    high,
                    enabled,
                } => {
                    let key = (tenant.clone(), low.clone(), high.clone());
                    if *enabled {
                        self.partitions.insert(key);
                    } else {
                        self.partitions.remove(&key);
                    }
                    self.record(&event, b"PARTITION_CHANGE", None);
                    receipt.partition_changes += 1;
                }
                SimEvent::Message(message) => {
                    let partitioned = message.sender != message.recipient
                        && self.partitions.contains(&link_key(
                            &message.tenant,
                            &message.sender,
                            &message.recipient,
                        )?);
                    if partitioned {
                        self.record(&event, b"PARTITION_DROP", None);
                        receipt.dropped_messages += 1;
                        continue;
                    }
                    let roll = self.draw_percent();
                    let drop_bound = u16::from(self.policy.drop_percent);
                    let corrupt_bound = drop_bound + u16::from(self.policy.corrupt_percent);
                    let duplicate_bound = corrupt_bound + u16::from(self.policy.duplicate_percent);
                    if roll < drop_bound {
                        self.record(&event, b"FAULT_DROP", Some(roll));
                        receipt.dropped_messages += 1;
                    } else if roll < corrupt_bound {
                        self.record(&event, b"FAULT_CORRUPT", Some(roll));
                        let mut corrupted = message.clone();
                        if corrupted.payload.is_empty() {
                            corrupted.payload.push(1);
                        } else {
                            corrupted.payload[0] ^= 1;
                        }
                        handler(&corrupted, DeliveryKind::InjectedCorruption, self);
                        receipt.delivered_callbacks += 1;
                        receipt.corrupted_callbacks += 1;
                    } else if roll < duplicate_bound {
                        self.record(&event, b"FAULT_DUPLICATE", Some(roll));
                        handler(message, DeliveryKind::Primary, self);
                        handler(message, DeliveryKind::InjectedDuplicate, self);
                        receipt.delivered_callbacks += 2;
                        receipt.duplicate_callbacks += 1;
                    } else {
                        self.record(&event, b"DELIVER", Some(roll));
                        handler(message, DeliveryKind::Primary, self);
                        receipt.delivered_callbacks += 1;
                    }
                }
            }
        }
        receipt.budget_exhausted = self.events.peek().is_some_and(|event| event.at_ms <= horizon_ms);
        if !receipt.budget_exhausted {
            self.now_ms = horizon_ms;
        }
        receipt.pending_events = self.events.len();
        receipt.current_time_ms = self.now_ms;
        receipt.transcript_sha256 = self.transcript_sha256;
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean() -> DeterministicSimulator {
        DeterministicSimulator::new(
            19,
            SimulatorLimits::default(),
            FaultPolicy {
                drop_percent: 0,
                duplicate_percent: 0,
                corrupt_percent: 0,
            },
        )
        .unwrap()
    }

    fn message(tenant: &str, sender: &str, recipient: &str, payload: &[u8]) -> SimMessage {
        SimMessage {
            tenant: tenant.into(),
            sender: sender.into(),
            recipient: recipient.into(),
            payload: payload.into(),
        }
    }

    #[test]
    fn equal_time_events_follow_monotonic_insertion_id() {
        let mut sim = clean();
        for payload in [b"A".as_slice(), b"B".as_slice(), b"C".as_slice()] {
            sim.schedule_message(10, message("t1", "a", "b", payload)).unwrap();
        }
        let mut observed = Vec::new();
        let r = sim.run_until(10, 10, |msg, kind, _| {
            assert_eq!(kind, DeliveryKind::Primary);
            observed.push(msg.payload.clone());
        }).unwrap();
        assert_eq!(observed, vec![b"A".to_vec(), b"B".to_vec(), b"C".to_vec()]);
        assert_eq!(r.processed_events, 3);
        assert!(!r.budget_exhausted);
        assert!(r.is_simulation_only());
    }

    #[test]
    fn future_event_is_not_consumed_at_shorter_horizon() {
        let mut sim = clean();
        sim.schedule_message(20, message("t1", "a", "b", b"future")).unwrap();
        let early = sim.run_until(10, 5, |_, _, _| panic!("future event delivered")).unwrap();
        assert_eq!(early.pending_events, 1);
        assert_eq!(early.current_time_ms, 10);
        assert_eq!(sim.queued_bytes(), 6);
        let later = sim.run_until(20, 5, |m, _, _| assert_eq!(m.payload, b"future")).unwrap();
        assert_eq!(later.delivered_callbacks, 1);
        assert_eq!(later.pending_events, 0);
    }

    #[test]
    fn partition_is_direction_independent_and_tenant_scoped() {
        let mut sim = clean();
        sim.schedule_partition(0, "tenant1", "b", "a", true).unwrap();
        sim.schedule_message(1, message("tenant1", "a", "b", b"blocked")).unwrap();
        sim.schedule_message(1, message("tenant2", "a", "b", b"allowed")).unwrap();
        let mut seen = Vec::new();
        let r = sim.run_until(2, 10, |m, _, _| seen.push(m.tenant.clone())).unwrap();
        assert_eq!(seen, vec!["tenant2"]);
        assert_eq!(r.dropped_messages, 1);
        assert!(sim.is_partitioned("tenant1", "a", "b").unwrap());
        assert!(!sim.is_partitioned("tenant2", "a", "b").unwrap());
    }

    #[test]
    fn partition_recovery_delivers_after_reconnection() {
        let mut sim = clean();
        sim.schedule_partition(0, "t1", "a", "b", true).unwrap();
        sim.schedule_partition(5, "t1", "a", "b", false).unwrap();
        sim.schedule_message(4, message("t1", "a", "b", b"lost")).unwrap();
        sim.schedule_message(6, message("t1", "a", "b", b"ok")).unwrap();
        let mut values = Vec::new();
        let r = sim.run_until(7, 10, |m, _, _| values.push(m.payload.clone())).unwrap();
        assert_eq!(r.partition_changes, 2);
        assert_eq!(r.dropped_messages, 1);
        assert_eq!(values, vec![b"ok".to_vec()]);
        assert!(!sim.is_partitioned("t1", "a", "b").unwrap());
    }

    #[test]
    fn duplicate_fault_requires_consumer_deduplication() {
        let mut sim = DeterministicSimulator::new(
            1, SimulatorLimits::default(),
            FaultPolicy { drop_percent: 0, duplicate_percent: 100, corrupt_percent: 0 }
        ).unwrap();
        sim.schedule_message(1, message("t1", "a", "b", b"test")).unwrap();
        let mut kinds = Vec::new();
        let r = sim.run_until(1, 1, |_, kind, _| kinds.push(kind)).unwrap();
        assert_eq!(kinds, vec![DeliveryKind::Primary, DeliveryKind::InjectedDuplicate]);
        assert_eq!(r.duplicate_callbacks, 1);
        assert_eq!(r.delivered_callbacks, 2);
    }

    #[test]
    fn corruption_fault_changes_payload_without_editing_enqueued_input() {
        let mut sim = DeterministicSimulator::new(
            1, SimulatorLimits::default(),
            FaultPolicy { drop_percent: 0, duplicate_percent: 0, corrupt_percent: 100 }
        ).unwrap();
        let original = message("t1", "a", "b", b"truth");
        sim.schedule_message(1, original.clone()).unwrap();
        let r = sim.run_until(1, 1, |m, kind, _| {
            assert_eq!(kind, DeliveryKind::InjectedCorruption);
            assert_ne!(m.payload, original.payload);
        }).unwrap();
        assert_eq!(original.payload, b"truth");
        assert_eq!(r.corrupted_callbacks, 1);
    }

    #[test]
    fn deterministic_drop_never_invokes_handler() {
        let mut sim = DeterministicSimulator::new(
            1, SimulatorLimits::default(),
            FaultPolicy { drop_percent: 100, duplicate_percent: 0, corrupt_percent: 0 }
        ).unwrap();
        sim.schedule_message(1, message("t1", "a", "b", b"discard")).unwrap();
        let r = sim.run_until(1, 1, |_, _, _| panic!("dropped message delivered")).unwrap();
        assert_eq!(r.dropped_messages, 1);
        assert_eq!(r.delivered_callbacks, 0);
    }

    #[test]
    fn segmented_and_one_shot_execution_have_the_same_transcript() {
        let mut a = clean();
        for (time, data) in [(1, b"a"), (5, b"b"), (8, b"c")] {
            a.schedule_message(time, message("t1", "x", "y", data)).unwrap();
        }
        let mut b = a.clone();
        let mut first = Vec::new();
        a.run_until(8, 10, |m, _, _| first.push(m.payload.clone())).unwrap();
        let mut second = Vec::new();
        b.run_until(4, 10, |m, _, _| second.push(m.payload.clone())).unwrap();
        b.run_until(8, 10, |m, _, _| second.push(m.payload.clone())).unwrap();
        assert_eq!(first, second);
        assert_eq!(a.trace_digest(), b.trace_digest());
        assert_eq!(a.now_ms(), b.now_ms());
    }

    #[test]
    fn callback_can_enqueue_but_budget_stops_zero_delay_feedback() {
        let mut sim = clean();
        sim.schedule_message(0, message("t1", "a", "b", b"loop")).unwrap();
        let r = sim.run_until(0, 5, |m, _, s| {
            s.schedule_message(0, m.clone()).unwrap();
        }).unwrap();
        assert_eq!(r.processed_events, 5);
        assert_eq!(r.pending_events, 1);
        assert!(r.budget_exhausted);
        assert_eq!(r.current_time_ms, 0);
        assert_eq!(r.delivered_callbacks, 5);
    }

    #[test]
    fn over_capacity_rejection_leaves_state_unchanged() {
        let mut sim = DeterministicSimulator::new(
            1,
            SimulatorLimits { max_pending_events: 1, max_queued_bytes: 5, max_payload_bytes: 5, max_steps_per_run: 10 },
            FaultPolicy { drop_percent: 0, duplicate_percent: 0, corrupt_percent: 0 },
        ).unwrap();
        sim.schedule_message(1, message("t1", "a", "b", b"abcde")).unwrap();
        let digest = sim.trace_digest();
        let bytes = sim.queued_bytes();
        assert_eq!(
            sim.schedule_message(1, message("t1", "a", "b", b"x")),
            Err(SimError::QueueLimit)
        );
        assert_eq!(sim.pending_events(), 1);
        assert_eq!(sim.queued_bytes(), bytes);
        assert_eq!(sim.trace_digest(), digest);
        sim.run_until(1, 1, |m, _, _| assert_eq!(m.payload, b"abcde")).unwrap();
    }

    #[test]
    fn invalid_configuration_and_identifiers_fail_closed() {
        assert!(matches!(
            DeterministicSimulator::new(
                1, SimulatorLimits::default(),
                FaultPolicy { drop_percent: 80, duplicate_percent: 30, corrupt_percent: 0 }
            ),
            Err(SimError::InvalidConfiguration)
        ));
        let mut sim = clean();
        assert_eq!(
            sim.schedule_message(0, message("../tenant", "a", "b", b"data")),
            Err(SimError::InvalidIdentity)
        );
        assert_eq!(
            sim.schedule_partition(0, "t1", "a", "a", true),
            Err(SimError::InvalidLink)
        );
        assert_eq!(sim.pending_events(), 0);
    }

    #[test]
    fn invalid_horizon_does_not_consume_any_message() {
        let mut sim = clean();
        sim.schedule_message(20, message("t1", "a", "b", b"kept")).unwrap();
        let early = sim.run_until(10, 1, |_, _, _| {}).unwrap();
        assert_eq!(early.current_time_ms, 10);
        assert_eq!(sim.run_until(9, 1, |_, _, _| {}), Err(SimError::InvalidHorizon));
        assert_eq!(sim.pending_events(), 1);
        assert_eq!(sim.now_ms(), 10);
    }

    #[test]
    fn clock_overflow_and_sequence_exhaustion_are_rejected() {
        let mut sim = clean();
        sim.run_until(u64::MAX, 1, |_, _, _| {}).unwrap();
        assert_eq!(
            sim.schedule_message(1, message("t1", "a", "b", b"x")),
            Err(SimError::ClockOverflow)
        );
        assert_eq!(sim.pending_events(), 0);
        let mut other = clean();
        other.next_id = u64::MAX;
        assert_eq!(
            other.schedule_message(0, message("t1", "a", "b", b"x")),
            Err(SimError::SequenceExhausted)
        );
        assert_eq!(other.pending_events(), 0);
    }

    #[test]
    fn transcript_is_deterministic_and_only_a_simulation_fingerprint() {
        let mut a = clean();
        let mut b = clean();
        for sim in [&mut a, &mut b] {
            sim.schedule_message(3, message("t1", "a", "b", b"payload")).unwrap();
        }
        let ra = a.run_until(3, 10, |_, _, _| {}).unwrap();
        let rb = b.run_until(3, 10, |_, _, _| {}).unwrap();
        assert_eq!(ra.transcript_sha256, rb.transcript_sha256);
        assert_ne!(ra.transcript_sha256, [0; 32]);
        assert!(ra.is_simulation_only());
        assert!(rb.is_simulation_only());
    }

    #[test]
    fn oversize_payload_is_rejected_before_queue_modification() {
        let mut sim = clean();
        let initial = sim.trace_digest();
        assert_eq!(
            sim.schedule_message(1, message("t1", "a", "b", &vec![0; 65537])),
            Err(SimError::PayloadLimit)
        );
        assert_eq!(sim.pending_events(), 0);
        assert_eq!(sim.queued_bytes(), 0);
        assert_eq!(sim.trace_digest(), initial);
    }
}
