use ocaf_runtime::metrics::bounded_telemetry::{bounded_channel, TelemetryError, TelemetrySample};
use std::time::{Duration, Instant};

#[tokio::test]
async fn stalled_sink_saturates_then_recovers_without_silent_loss() {
    let (sender, mut receiver) = bounded_channel(4).unwrap();
    for sequence in 0..4 {
        sender
            .try_record(TelemetrySample {
                sequence,
                latency_us: 10,
            })
            .unwrap();
    }
    assert_eq!(
        sender.try_record(TelemetrySample {
            sequence: 4,
            latency_us: 10
        }),
        Err(TelemetryError::Backpressure)
    );
    assert_eq!(sender.rejected(), 1);
    for sequence in 0..4 {
        assert_eq!(receiver.recv().await.unwrap().sequence, sequence);
    }
    sender
        .try_record(TelemetrySample {
            sequence: 5,
            latency_us: 10,
        })
        .unwrap();
    assert_eq!(receiver.recv().await.unwrap().sequence, 5);
    drop(receiver);
    assert_eq!(
        sender.try_record(TelemetrySample {
            sequence: 6,
            latency_us: 10
        }),
        Err(TelemetryError::Closed)
    );
    assert_eq!(sender.rejected(), 2);
}

#[test]
fn rejects_unbounded_or_empty_queue() {
    assert!(matches!(
        bounded_channel(0),
        Err(TelemetryError::InvalidCapacity)
    ));
    assert!(matches!(
        bounded_channel(65_537),
        Err(TelemetryError::InvalidCapacity)
    ));
}

#[tokio::test]
async fn repeated_sink_outage_recovery_is_measured() {
    let seconds: u64 = std::env::var("OCAF_STAGE041_TELEMETRY_SOAK_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    assert!(seconds == 1 || seconds == 3600);
    let (sender, mut receiver) = bounded_channel(32).unwrap();
    let start = Instant::now();
    let mut accepted = 0u64;
    let mut rejected = 0u64;
    let mut drained = 0u64;
    while start.elapsed() < Duration::from_secs(seconds) {
        for _ in 0..64 {
            let sample = TelemetrySample {
                sequence: accepted + rejected,
                latency_us: 1,
            };
            match sender.try_record(sample) {
                Ok(()) => accepted += 1,
                Err(TelemetryError::Backpressure) => rejected += 1,
                other => panic!("unexpected enqueue result: {other:?}"),
            }
        }
        for _ in 0..16 {
            if let Some(sample) = receiver.recv().await {
                assert_eq!(sample.latency_us, 1);
                drained += 1;
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    for _ in 0..(accepted - drained) {
        receiver.recv().await.unwrap();
        drained += 1;
    }
    assert_eq!(sender.rejected(), rejected);
    assert_eq!(accepted, drained);
    assert!(accepted > 0 && rejected > 0);
    println!("accepted={accepted} rejected={rejected} duration_seconds={seconds} scope=in_process_no_otlp");
}
