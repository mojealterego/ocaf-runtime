pub mod auth;
pub mod cluster;
pub mod coordinator;
pub mod executor;
pub mod jobs;
pub mod metrics;
pub mod omas_policy;
pub mod omas_signed;
pub mod ops;
pub mod registry;
pub mod rpc;
pub mod runtime;
pub mod sink;

#[cfg(feature = "zenoh")]
pub mod transport;

pub mod transport_policy;

pub mod agent_fleet;

pub mod omas_server;

pub mod omat_server;

pub mod stage043_signed_manifest;

pub mod stage043_store;

pub mod stage043_signed_agent;

pub mod microvm_lifecycle;

pub mod firecracker_ebpf_contract;

pub mod stage_ledger;

pub mod postgres_ha_contract;

pub mod stage040_interhost_contract;

pub mod reliability_recovery;

pub mod stage041_recovery_contract;

pub mod stage043_agent_store;
pub mod stage043_mtls_admission;

pub mod stage043_omat_route;

#[cfg(feature = "zenoh")]
pub mod stage043_omat_discovery;
#[cfg(feature = "zenoh")]
pub mod stage043_omat_revoke_zenoh;
#[cfg(feature = "zenoh")]
pub mod stage043_omat_zenoh;
pub mod stage044_oauth;
pub mod stage044_omat_authz;
pub mod stage044_secret_broker;

pub mod stage044_oidc;

pub mod stage044_vault;

pub mod stage044_vault_broker;

pub mod stage044_provider_refresh;

pub mod sasos_capability;

pub mod sasos_service_adapters;

pub mod document_state;

pub mod document_store;

pub mod governance_contract;

pub mod retrieval_quality;

pub mod governance_journal;

pub mod finops_ledger;

pub mod omnidev_workflow;

pub mod compliance_evidence;

pub mod contract_obligations;

pub mod mcp_registry_integrity;

pub mod claim_ledger_verifier;

pub mod document_function_dag;

pub mod model_router;

pub mod voice_narrative_firewall;

pub mod collaboration_tracker;

pub mod agentic_prompt_dsl;

pub mod compound_reasoning;
