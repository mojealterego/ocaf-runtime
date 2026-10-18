//! Strict, bounded W3C Trace Context `traceparent` version 00 parsing.
//!
//! This is a transport-neutral building block for OCAF trace propagation.
//! It does not export OTLP or claim end-to-end trace propagation.

use std::fmt;
use std::fmt::Write as _;

/// An authenticated transport still has to bind the trace to the calling tenant.
/// A trace identifier by itself must never grant access to resources.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceParent {
    trace_id: [u8; 16],
    parent_id: [u8; 8],
    trace_flags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceParentError {
    InvalidFormat,
    UnsupportedVersion,
    InvalidTraceId,
    InvalidParentId,
    InvalidFlags,
}

impl fmt::Display for TraceParentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidFormat => "invalid traceparent header format",
            Self::UnsupportedVersion => "unsupported traceparent version",
            Self::InvalidTraceId => "invalid trace ID",
            Self::InvalidParentId => "invalid parent ID",
            Self::InvalidFlags => "invalid trace flags",
        })
    }
}

impl std::error::Error for TraceParentError {}

impl TraceParent {
    pub fn new(
        trace_id: [u8; 16],
        parent_id: [u8; 8],
        trace_flags: u8,
    ) -> Result<Self, TraceParentError> {
        if trace_id.iter().all(|byte| *byte == 0) {
            return Err(TraceParentError::InvalidTraceId);
        }
        if parent_id.iter().all(|byte| *byte == 0) {
            return Err(TraceParentError::InvalidParentId);
        }
        Ok(Self {
            trace_id,
            parent_id,
            trace_flags,
        })
    }

    /// Accepts exactly version 00, its fixed 55-byte representation and
    /// lowercase hexadecimal, rejecting unsupported extensions explicitly.
    pub fn parse(value: &str) -> Result<Self, TraceParentError> {
        let raw = value.as_bytes();
        if raw.len() != 55 || raw[2] != b'-' || raw[35] != b'-' || raw[52] != b'-' {
            return Err(TraceParentError::InvalidFormat);
        }
        if &raw[0..2] != b"00" {
            return Err(TraceParentError::UnsupportedVersion);
        }
        let trace_id = decode_hex::<16>(&raw[3..35])
            .ok_or(TraceParentError::InvalidTraceId)?;
        let parent_id = decode_hex::<8>(&raw[36..52])
            .ok_or(TraceParentError::InvalidParentId)?;
        let flags = decode_hex::<1>(&raw[53..55])
            .ok_or(TraceParentError::InvalidFlags)?[0];
        Self::new(trace_id, parent_id, flags)
    }

    pub fn trace_id(&self) -> [u8; 16] {
        self.trace_id
    }

    pub fn parent_id(&self) -> [u8; 8] {
        self.parent_id
    }

    pub fn trace_flags(&self) -> u8 {
        self.trace_flags
    }

    pub fn header_value(&self) -> String {
        let mut value = String::with_capacity(55);
        value.push_str("00-");
        append_hex(&mut value, &self.trace_id);
        value.push('-');
        append_hex(&mut value, &self.parent_id);
        value.push('-');
        append_hex(&mut value, &[self.trace_flags]);
        value
    }
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn decode_hex<const N: usize>(raw: &[u8]) -> Option<[u8; N]> {
    if raw.len() != N * 2 {
        return None;
    }
    let mut decoded = [0u8; N];
    for (index, byte) in decoded.iter_mut().enumerate() {
        *byte = (hex_nibble(raw[index * 2])? << 4) | hex_nibble(raw[index * 2 + 1])?;
    }
    Some(decoded)
}

fn append_hex(target: &mut String, bytes: &[u8]) {
    for byte in bytes {
        write!(target, "{byte:02x}").expect("writing to String cannot fail");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    #[test]
    fn w3c_example_round_trip() {
        let parsed = TraceParent::parse(EXAMPLE).unwrap();
        assert_eq!(parsed.header_value(), EXAMPLE);
        assert_eq!(parsed.trace_flags(), 1);
        assert_eq!(parsed.trace_id()[0], 0x4b);
        assert_eq!(parsed.parent_id()[0], 0x00);
    }

    #[test]
    fn rejects_all_zero_identifiers() {
        assert_eq!(
            TraceParent::parse("00-00000000000000000000000000000000-00f067aa0ba902b7-01"),
            Err(TraceParentError::InvalidTraceId),
        );
        assert_eq!(
            TraceParent::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01"),
            Err(TraceParentError::InvalidParentId),
        );
        assert_eq!(
            TraceParent::new([0; 16], [1; 8], 1),
            Err(TraceParentError::InvalidTraceId),
        );
    }

    #[test]
    fn rejects_unsupported_or_reserved_versions() {
        for version in ["01", "ff", "FF", "a0"] {
            let header = format!("{version}{}", &EXAMPLE[2..]);
            assert_eq!(
                TraceParent::parse(&header),
                Err(TraceParentError::UnsupportedVersion),
            );
        }
    }

    #[test]
    fn rejects_noncanonical_and_malformed_headers() {
        for value in [
            format!(" {EXAMPLE}"),
            format!("{EXAMPLE} "),
            EXAMPLE.to_ascii_uppercase(),
            EXAMPLE.replace('-', ":"),
            format!("{EXAMPLE}-extra"),
            EXAMPLE[..EXAMPLE.len() - 1].to_owned(),
        ] {
            assert!(TraceParent::parse(&value).is_err(), "accepted {value:?}");
        }
    }

    #[test]
    fn rejects_invalid_hex_in_each_component() {
        let trace_id = EXAMPLE.replacen("4bf9", "4bg9", 1);
        assert_eq!(
            TraceParent::parse(&trace_id),
            Err(TraceParentError::InvalidTraceId),
        );
        let parent_id = EXAMPLE.replacen("00f0", "0xf0", 1);
        assert_eq!(
            TraceParent::parse(&parent_id),
            Err(TraceParentError::InvalidParentId),
        );
        let flags = format!("{}gg", &EXAMPLE[..EXAMPLE.len() - 2]);
        assert_eq!(
            TraceParent::parse(&flags),
            Err(TraceParentError::InvalidFlags),
        );
    }
}
