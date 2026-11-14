//! Bounded GGUF *header* parser (OCAF architecture reference, pages 28-29).
//!
//! Validates only the fixed 24-byte preamble: magic, v2/v3 and bounded counts.
//! This does NOT parse tensor descriptors, metadata, alignment, offsets,
//! weights, quantization or checksums. Header acceptance is NOT model approval.
//! It performs no allocation proportional to untrusted GGUF counter values.

pub const GGUF_HEADER_BYTES: usize = 24;
pub const GGUF_MAX_TENSORS: u64 = 1_000_000;
pub const GGUF_MAX_METADATA_PAIRS: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgufHeaderError {
    BufferUnderflow,
    InvalidMagic,
    UnsupportedVersion(u32),
    TensorCountLimit(u64),
    MetadataCountLimit(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GgufHeader {
    pub version: u32,
    pub tensor_count: u64,
    pub metadata_kv_count: u64,
}

pub struct SafeGgufHeaderParser;

impl SafeGgufHeaderParser {
    pub fn parse_header(buffer: &[u8]) -> Result<GgufHeader, GgufHeaderError> {
        if buffer.len() < GGUF_HEADER_BYTES {
            return Err(GgufHeaderError::BufferUnderflow);
        }
        if !buffer.starts_with(b"GGUF") {
            return Err(GgufHeaderError::InvalidMagic);
        }
        let version = u32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
        if version != 2 && version != 3 {
            return Err(GgufHeaderError::UnsupportedVersion(version));
        }

        // The length guard above makes these fixed-size copies safe and avoids
        // unchecked indexing or allocations derived from untrusted counts.
        let mut tensor_bytes = [0u8; 8];
        tensor_bytes.copy_from_slice(&buffer[8..16]);
        let tensor_count = u64::from_le_bytes(tensor_bytes);
        if tensor_count > GGUF_MAX_TENSORS {
            return Err(GgufHeaderError::TensorCountLimit(tensor_count));
        }
        let mut metadata_bytes = [0u8; 8];
        metadata_bytes.copy_from_slice(&buffer[16..24]);
        let metadata_kv_count = u64::from_le_bytes(metadata_bytes);
        if metadata_kv_count > GGUF_MAX_METADATA_PAIRS {
            return Err(GgufHeaderError::MetadataCountLimit(metadata_kv_count));
        }
        Ok(GgufHeader {
            version,
            tensor_count,
            metadata_kv_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(version: u32, tensors: u64, metadata: u64) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"GGUF");
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(&tensors.to_le_bytes());
        bytes.extend_from_slice(&metadata.to_le_bytes());
        bytes
    }

    #[test]
    fn parses_both_supported_versions_without_tensor_allocation() {
        assert_eq!(
            SafeGgufHeaderParser::parse_header(&fixture(2, 100, 20)),
            Ok(GgufHeader { version: 2, tensor_count: 100, metadata_kv_count: 20 })
        );
        assert_eq!(
            SafeGgufHeaderParser::parse_header(&fixture(3, 1, 0)),
            Ok(GgufHeader { version: 3, tensor_count: 1, metadata_kv_count: 0 })
        );
    }

    #[test]
    fn all_short_headers_fail_closed_without_panics() {
        let valid = fixture(3, 2, 3);
        for length in 0..GGUF_HEADER_BYTES {
            assert_eq!(
                SafeGgufHeaderParser::parse_header(&valid[..length]),
                Err(GgufHeaderError::BufferUnderflow),
                "short header of length {length} accepted"
            );
        }
    }

    #[test]
    fn rejects_incorrect_and_big_endian_magic() {
        let mut invalid = fixture(3, 2, 3);
        invalid[0..4].copy_from_slice(b"FUGG");
        assert_eq!(
            SafeGgufHeaderParser::parse_header(&invalid),
            Err(GgufHeaderError::InvalidMagic)
        );
        invalid[0..4].copy_from_slice(b"GGUF");
        assert!(SafeGgufHeaderParser::parse_header(&invalid).is_ok());
    }

    #[test]
    fn rejects_unsupported_versions() {
        for version in [0, 1, 4, u32::MAX] {
            assert_eq!(
                SafeGgufHeaderParser::parse_header(&fixture(version, 0, 0)),
                Err(GgufHeaderError::UnsupportedVersion(version))
            );
        }
    }

    #[test]
    fn rejects_excessive_tensor_count_before_any_allocation() {
        let value = GGUF_MAX_TENSORS + 1;
        assert_eq!(
            SafeGgufHeaderParser::parse_header(&fixture(3, value, 0)),
            Err(GgufHeaderError::TensorCountLimit(value))
        );
        assert_eq!(
            SafeGgufHeaderParser::parse_header(&fixture(3, u64::MAX, 0)),
            Err(GgufHeaderError::TensorCountLimit(u64::MAX))
        );
    }

    #[test]
    fn rejects_excessive_metadata_pair_count() {
        let value = GGUF_MAX_METADATA_PAIRS + 1;
        assert_eq!(
            SafeGgufHeaderParser::parse_header(&fixture(2, 0, value)),
            Err(GgufHeaderError::MetadataCountLimit(value))
        );
    }

    #[test]
    fn little_endian_counts_preserve_64_bit_values_within_bounds() {
        let tensors = 65_536;
        let metadata = 100_000;
        let result = SafeGgufHeaderParser::parse_header(&fixture(3, tensors, metadata)).unwrap();
        assert_eq!(result.tensor_count, tensors);
        assert_eq!(result.metadata_kv_count, metadata);
    }

    #[test]
    fn boundary_values_are_admissible_but_not_model_validation() {
        let h = SafeGgufHeaderParser::parse_header(
            &fixture(3, GGUF_MAX_TENSORS, GGUF_MAX_METADATA_PAIRS)
        ).unwrap();
        assert_eq!(h.tensor_count, GGUF_MAX_TENSORS);
        assert_eq!(h.metadata_kv_count, GGUF_MAX_METADATA_PAIRS);
    }

    #[test]
    fn trailing_bytes_do_not_change_fixed_header_decoding() {
        let mut file = fixture(3, 1, 1);
        let baseline = SafeGgufHeaderParser::parse_header(&file).unwrap();
        file.extend_from_slice(&[0xff; 100]);
        assert_eq!(SafeGgufHeaderParser::parse_header(&file), Ok(baseline));
    }

    #[test]
    fn deterministic_malformed_input_corpus_never_panics() {
        let mut state = 1u64;
        for length in 0..128usize {
            let mut bytes = vec![0u8; length];
            for byte in &mut bytes {
                state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                *byte = (state >> 24) as u8;
            }
            // Parsing arbitrary bytes must return an error or a bounded header.
            if let Ok(header) = SafeGgufHeaderParser::parse_header(&bytes) {
                assert!(matches!(header.version, 2 | 3));
                assert!(header.tensor_count <= GGUF_MAX_TENSORS);
                assert!(header.metadata_kv_count <= GGUF_MAX_METADATA_PAIRS);
            }
        }
    }
}
