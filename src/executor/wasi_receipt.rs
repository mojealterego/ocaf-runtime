//! Cryptographically authenticated execution evidence for the restricted
//! WASI path. Receipts attest OCAF-observed inputs and outcomes, NOT that a
//! Wasm guest is semantically correct or that an external effect was applied.
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::executor::ToolError;

type HmacSha256 = Hmac<Sha256>;
const RECEIPT_DOMAIN: &[u8] = b"OCAF-WASI-RECEIPT-V1\0";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReceipt {
    pub schema_version: u8,
    pub tenant_id: String,
    pub agent_id: String,
    pub tool_name: String,
    pub correlation_id: String,
    pub module_sha256_hex: String,
    pub input_sha256_hex: String,
    pub output_sha256_hex: String,
    pub elapsed_ms: u64,
    pub completed_unix_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedExecutionReceipt {
    pub receipt: ExecutionReceipt,
    pub hmac_sha256_hex: String,
}

/// A dedicated per-deployment 256-bit receipt MAC key. Provision out of Git.
/// A receipt requires independent verifier possession of the same key.
pub struct ReceiptSigner {
    key: [u8; 32],
}

impl ReceiptSigner {
    pub fn new(key: [u8; 32]) -> Result<Self, ToolError> {
        if key == [0; 32] {
            return Err(ToolError::Validation(
                "zero receipt signing key rejected".into(),
            ));
        }
        Ok(Self { key })
    }

    pub fn sign(&self, receipt: ExecutionReceipt) -> Result<SignedExecutionReceipt, ToolError> {
        if receipt.schema_version != 1 {
            return Err(ToolError::Validation("unsupported receipt schema".into()));
        }
        let content = serde_json::to_vec(&receipt)
            .map_err(|_| ToolError::Execution("cannot serialize WASI receipt".into()))?;
        let mut mac = HmacSha256::new_from_slice(&self.key)
            .map_err(|_| ToolError::Execution("receipt HMAC setup failed".into()))?;
        mac.update(RECEIPT_DOMAIN);
        mac.update(&content);
        let signature = mac.finalize().into_bytes();
        let mut hex = String::with_capacity(64);
        use std::fmt::Write as _;
        for value in signature {
            write!(&mut hex, "{value:02x}").expect("hex String");
        }
        Ok(SignedExecutionReceipt {
            receipt,
            hmac_sha256_hex: hex,
        })
    }

    pub fn verify(&self, signed: &SignedExecutionReceipt) -> Result<(), ToolError> {
        if signed.hmac_sha256_hex.len() != 64
            || !signed
                .hmac_sha256_hex
                .bytes()
                .all(|v| v.is_ascii_hexdigit())
            || signed
                .hmac_sha256_hex
                .bytes()
                .any(|v| v.is_ascii_uppercase())
        {
            return Err(ToolError::Validation("invalid receipt MAC encoding".into()));
        }
        let mut decoded = [0u8; 32];
        for (index, pair) in signed
            .hmac_sha256_hex
            .as_bytes()
            .as_chunks::<2>().0.iter()
            .enumerate()
        {
            decoded[index] = u8::from_str_radix(
                std::str::from_utf8(pair)
                    .map_err(|_| ToolError::Validation("invalid receipt MAC".into()))?,
                16,
            )
            .map_err(|_| ToolError::Validation("invalid receipt MAC".into()))?;
        }
        let content = serde_json::to_vec(&signed.receipt)
            .map_err(|_| ToolError::Validation("invalid receipt".into()))?;
        let mut mac = HmacSha256::new_from_slice(&self.key)
            .map_err(|_| ToolError::Execution("receipt HMAC setup failed".into()))?;
        mac.update(RECEIPT_DOMAIN);
        mac.update(&content);
        mac.verify_slice(&decoded)
            .map_err(|_| ToolError::Validation("WASI receipt MAC verification failed".into()))
    }
}
