//! Bounded in-process telemetry handoff. Explicit overflow is observable.
//! This is not an OTLP exporter or durable trace storage.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelemetrySample {
    pub sequence: u64,
    pub latency_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryError {
    InvalidCapacity,
    Backpressure,
    Closed,
}

#[derive(Clone)]
pub struct TelemetrySender {
    sender: mpsc::Sender<TelemetrySample>,
    rejected: Arc<AtomicU64>,
}

pub struct TelemetryReceiver(mpsc::Receiver<TelemetrySample>);

pub fn bounded_channel(
    capacity: usize,
) -> Result<(TelemetrySender, TelemetryReceiver), TelemetryError> {
    if !(1..=65_536).contains(&capacity) {
        return Err(TelemetryError::InvalidCapacity);
    }
    let (sender, receiver) = mpsc::channel(capacity);
    Ok((
        TelemetrySender {
            sender,
            rejected: Arc::new(AtomicU64::new(0)),
        },
        TelemetryReceiver(receiver),
    ))
}

impl TelemetrySender {
    /// Never blocks a workload on a failed telemetry sink. A failed enqueue
    /// is returned to the caller and counted for operator alerting.
    pub fn try_record(&self, sample: TelemetrySample) -> Result<(), TelemetryError> {
        match self.sender.try_send(sample) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.rejected.fetch_add(1, Ordering::Relaxed);
                Err(TelemetryError::Backpressure)
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.rejected.fetch_add(1, Ordering::Relaxed);
                Err(TelemetryError::Closed)
            }
        }
    }

    pub fn rejected(&self) -> u64 {
        self.rejected.load(Ordering::Relaxed)
    }
}

impl TelemetryReceiver {
    pub async fn recv(&mut self) -> Option<TelemetrySample> {
        self.0.recv().await
    }
}
