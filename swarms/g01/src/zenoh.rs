//! Reserved transport boundary for a later, separately tested Zenoh adapter.
//! The G01 executable has no path that opens a network session.

/// The registry requires Zenoh eventually, but local G01 stage work is not an
/// authorization to publish or subscribe. Every attempted transport call
/// fails closed until identity, ACL, replay, and live transport tests exist.
pub fn transport_status() -> Result<(), &'static str> {
    Err("G01 Zenoh transport is not implemented or authorized")
}
