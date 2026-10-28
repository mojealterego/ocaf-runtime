//! Asterion is a deterministic, local assistant for the OCAF stage ledger.
//! It has no model, live Zenoh connection, tool executor, or source access.

pub mod agent;
pub mod lifecycle;
pub mod security;
pub mod zenoh;

pub const AGENT_ID: &str = "OCAF-AGENT-001";
pub const DISPLAY_NAME: &str = "Asterion";
pub const GROUP_ID: &str = "G01";
pub const SPECIALIZATION: &str = "orchestration-runtime";

pub use ocaf_stage_ledger::{StageLedger, StageLedgerError, StageRecord, StageStatus};
