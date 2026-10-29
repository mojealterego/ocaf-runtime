//! Stage 334: operator-bound real HTTPS service adapters for the SASOS catalog.
//!
//! The 555 catalog records are declarations, NOT 555 running services. This
//! module executes only explicitly bound services through the existing
//! OAuth-authorized, Vault-backed, TLS-verified outbound broker. It does not
//! modify OMAt's fail-closed OSTc or WASI/Firecracker execution boundaries.
//! Ring0 tools are forbidden here pending separate sandbox admission.
use crate::stage044_oauth::AuthorizedGrant;
use crate::stage044_secret_broker::BrokerResponse;
use crate::stage044_vault_broker::VaultSecretBroker;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use thiserror::Error;

const MAX_BINDINGS: usize = 555;
const MAX_REQUEST_BYTES: usize = 65_536;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SasosAdapterError {
    #[error("invalid SASOS catalog or operator binding")]
    InvalidConfiguration,
    #[error("SASOS tool has no operator-approved backend binding")]
    Unbound,
    #[error("privileged SASOS tool requires separate sandbox attestation")]
    PrivilegedTool,
    #[error("arguments violate declared SASOS contract")]
    InvalidArguments,
    #[error("Vault-backed service execution failed")]
    BackendUnavailable,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SasosEntry {
    number: u16,
    tool_id: String,
    name: String,
    description: String,
    required_fields: Vec<String>,
    connected_modules: Vec<String>,
    declared_clearance: String,
    declared_runtime: String,
    status: String,
    executable: bool,
    imandra_verified: bool,
    measured_latency_microseconds: Option<u64>,
    zero_copy_verified: bool,
}

pub struct SasosCatalog {
    entries: BTreeMap<String, SasosEntry>,
}

impl SasosCatalog {
    pub fn from_json(raw: &str) -> Result<Self, SasosAdapterError> {
        if raw.len() > 2_000_000 {
            return Err(SasosAdapterError::InvalidConfiguration);
        }
        let rows: Vec<SasosEntry> =
            serde_json::from_str(raw).map_err(|_| SasosAdapterError::InvalidConfiguration)?;
        if rows.len() != 555 {
            return Err(SasosAdapterError::InvalidConfiguration);
        }
        let mut entries = BTreeMap::new();
        for (index, row) in rows.into_iter().enumerate() {
            let number = u16::try_from(index + 1)
                .map_err(|_| SasosAdapterError::InvalidConfiguration)?;
            let prefix = format!("SASOS_TOOL_{number:03}_");
            if row.number != number
                || !row.tool_id.starts_with(&prefix)
                || row.tool_id.len() <= prefix.len()
                || !row.tool_id.bytes().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_')
                || row.name.is_empty()
                || row.description.is_empty()
                || row.declared_runtime.is_empty()
                || row.declared_clearance.is_empty()
                || row.status != "CATALOG_ONLY"
                || row.executable
                || row.imandra_verified
                || row.zero_copy_verified
                || row.measured_latency_microseconds.is_some()
                || row.required_fields.len() > 128
                || row.connected_modules.len() > 128
                || row.required_fields.iter().any(|f| !safe_field(f))
                || row.connected_modules.iter().any(|f| !safe_field(f))
                || row.required_fields.iter().collect::<BTreeSet<_>>().len() != row.required_fields.len()
                || row.connected_modules.iter().collect::<BTreeSet<_>>().len() != row.connected_modules.len()
                || entries.insert(row.tool_id.clone(), row).is_some()
            {
                return Err(SasosAdapterError::InvalidConfiguration);
            }
        }
        Ok(Self { entries })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, tool_id: &str) -> bool {
        self.entries.contains_key(tool_id)
    }

    /// Counts *catalog declarations* only, never independently deployed routes.
    pub fn verified_active_count(&self) -> usize {
        self.entries.values().filter(|row| row.executable).count()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SasosServiceBinding {
    pub sasos_tool_id: String,
    pub route_id: String,
    pub broker_tool_id: String,
    pub max_body_bytes: usize,
}

struct BoundEntry {
    route_id: String,
    broker_tool_id: String,
    max_body_bytes: usize,
    required_fields: BTreeSet<String>,
}

pub struct SasosAdapterBindings {
    entries: BTreeMap<String, BoundEntry>,
}

pub struct SasosPreparedCall {
    pub route_id: String,
    pub broker_tool_id: String,
    pub body: Vec<u8>,
}

impl SasosAdapterBindings {
    /// Bind only operator-selected services; never accept model-selected URLs.
    pub fn new(
        catalog: &SasosCatalog,
        bindings: Vec<SasosServiceBinding>,
    ) -> Result<Self, SasosAdapterError> {
        if bindings.is_empty() || bindings.len() > MAX_BINDINGS {
            return Err(SasosAdapterError::InvalidConfiguration);
        }
        let mut entries = BTreeMap::new();
        for binding in bindings {
            if !safe_id(&binding.route_id)
                || !safe_id(&binding.broker_tool_id)
                || binding.max_body_bytes == 0
                || binding.max_body_bytes > MAX_REQUEST_BYTES
            {
                return Err(SasosAdapterError::InvalidConfiguration);
            }
            let declared = catalog
                .entries
                .get(&binding.sasos_tool_id)
                .ok_or(SasosAdapterError::InvalidConfiguration)?;
            // No arbitrary native/kernel privilege is granted by a broker.
            if declared.declared_clearance == "Ring0Kernel" {
                return Err(SasosAdapterError::PrivilegedTool);
            }
            if entries
                .insert(
                    binding.sasos_tool_id,
                    BoundEntry {
                        route_id: binding.route_id,
                        broker_tool_id: binding.broker_tool_id,
                        max_body_bytes: binding.max_body_bytes,
                        required_fields: declared.required_fields.iter().cloned().collect(),
                    },
                )
                .is_some()
            {
                return Err(SasosAdapterError::InvalidConfiguration);
            }
        }
        Ok(Self { entries })
    }

    /// Load operator-owned bindings from an absolute regular owner-only file.
    /// No runtime auto-discovery or implicit activation is performed.
    pub fn from_private_file(
        catalog: &SasosCatalog,
        path: &Path,
    ) -> Result<Self, SasosAdapterError> {
        let raw = crate::ops::secrets::read_private_text(path, MAX_REQUEST_BYTES)
            .map_err(|_| SasosAdapterError::InvalidConfiguration)?;
        let bindings: Vec<SasosServiceBinding> =
            serde_json::from_str(&raw).map_err(|_| SasosAdapterError::InvalidConfiguration)?;
        Self::new(catalog, bindings)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn prepare_call(
        &self,
        sasos_tool_id: &str,
        arguments: &Value,
    ) -> Result<SasosPreparedCall, SasosAdapterError> {
        let entry = self.entries.get(sasos_tool_id).ok_or(SasosAdapterError::Unbound)?;
        let fields = arguments.as_object().ok_or(SasosAdapterError::InvalidArguments)?;
        if fields.len() != entry.required_fields.len()
            || fields.iter().any(|(name, value)| {
                !entry.required_fields.contains(name) || value.is_null()
            })
        {
            return Err(SasosAdapterError::InvalidArguments);
        }
        let body = serde_json::to_vec(arguments).map_err(|_| SasosAdapterError::InvalidArguments)?;
        if body.len() > entry.max_body_bytes {
            return Err(SasosAdapterError::InvalidArguments);
        }
        Ok(SasosPreparedCall {
            route_id: entry.route_id.clone(),
            broker_tool_id: entry.broker_tool_id.clone(),
            body,
        })
    }

    /// Real network I/O: OAuth-authorized Vault broker resolves the operator
    /// route, decrypts the per-user secret in Vault, and sends HTTPS to the
    /// pinned public endpoint. Non-2xx responses are never reported as success.
    pub fn execute(
        &self,
        broker: &VaultSecretBroker,
        grant: &AuthorizedGrant,
        sasos_tool_id: &str,
        arguments: &Value,
    ) -> Result<BrokerResponse, SasosAdapterError> {
        let call = self.prepare_call(sasos_tool_id, arguments)?;
        let response = broker
            .execute(grant, &call.route_id, &call.broker_tool_id, &call.body)
            .map_err(|_| SasosAdapterError::BackendUnavailable)?;
        if !(200..300).contains(&response.status) {
            return Err(SasosAdapterError::BackendUnavailable);
        }
        Ok(response)
    }
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value != "."
        && value != ".."
        && value.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}

fn safe_field(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}
