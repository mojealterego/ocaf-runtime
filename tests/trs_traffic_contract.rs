//! Contract tests for bounded TRS traffic shaping.
//! This is a queue/packet-shape component, not an anonymity proof.
use ocaf_runtime::trs_traffic::{TrafficConfig, TrafficError, TrafficQueue};

#[test]
fn fifo_and_fixed_packet_size() {
    let mut q = TrafficQueue::new(TrafficConfig { packet_size: 32, capacity: 2 }).unwrap();
    q.enqueue(vec![1; 32]).unwrap();
    q.enqueue(vec![2; 32]).unwrap();
    assert_eq!(q.len(), 2);
    assert_eq!(q.next_packet().unwrap(), vec![1; 32]);
    assert_eq!(q.next_packet().unwrap(), vec![2; 32]);
    assert_eq!(q.len(), 0);
}
#[test]
fn rejects_wrong_size_and_exhaustion_without_mutating_queue() {
    let mut q = TrafficQueue::new(TrafficConfig { packet_size: 16, capacity: 1 }).unwrap();
    assert_eq!(q.enqueue(vec![0; 15]), Err(TrafficError::InvalidPacketSize));
    q.enqueue(vec![7; 16]).unwrap();
    assert_eq!(q.enqueue(vec![8; 16]), Err(TrafficError::QueueFull));
    assert_eq!(q.next_packet().unwrap(), vec![7; 16]);
}
#[test]
fn validates_bounds_and_produces_random_length_matched_chaff() {
    assert!(matches!(TrafficQueue::new(TrafficConfig { packet_size: 0, capacity: 1 }), Err(TrafficError::InvalidConfig)));
    assert!(matches!(TrafficQueue::new(TrafficConfig { packet_size: 32, capacity: 0 }), Err(TrafficError::InvalidConfig)));
    let mut q = TrafficQueue::new(TrafficConfig { packet_size: 32, capacity: 1 }).unwrap();
    let a = q.next_packet().unwrap();
    let b = q.next_packet().unwrap();
    assert_eq!(a.len(), 32);
    assert_eq!(b.len(), 32);
    assert_ne!(a, b);
}
#[test]
fn packet_timing_is_positive_and_rejects_invalid_rate() {
    use ocaf_runtime::trs_traffic::exponential_delay_seconds;
    assert!(exponential_delay_seconds(2.0).unwrap().is_finite());
    assert!(exponential_delay_seconds(2.0).unwrap() > 0.0);
    assert_eq!(exponential_delay_seconds(0.0), Err(TrafficError::InvalidRate));
    assert_eq!(exponential_delay_seconds(f64::NAN), Err(TrafficError::InvalidRate));
}
