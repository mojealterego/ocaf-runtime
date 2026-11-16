#![forbid(unsafe_code)]
//! Offline-only Rust UniFFI validation surface for Android / iOS.
//! Source of truth is the existing OCAF src/builder_core.rs, not a fork.
//! This FFI does not authenticate users or enable remote effects.

#[path = "../../../src/builder_core.rs"]
mod canonical_builder_core;

use canonical_builder_core::{compile, from_dsl, to_dsl};
use thiserror::Error;

uniffi::setup_scaffolding!();

#[derive(Debug, Error, uniffi::Error)]
pub enum LocalBuilderError {
    #[error("OCAF local JSON DSL validation rejected")]
    InvalidProject,
}

/// Compiles an inert, bounded execution plan; never executes tools or models.
#[uniffi::export]
pub fn validate_local_json_dsl(source: String) -> Result<String, LocalBuilderError> {
    let graph = from_dsl(&source, "local").map_err(|_| LocalBuilderError::InvalidProject)?;
    let plan = compile(&graph, "local").map_err(|_| LocalBuilderError::InvalidProject)?;
    serde_json::to_string(&serde_json::json!({
        "project_sha256": plan.project_sha256,
        "ordered_nodes": plan.ordered_nodes,
        "requested_tools": plan.requested_tools,
        "max_cost_minor": plan.max_cost_minor,
        "effects_executed": false,
        "namespace": "local"
    })).map_err(|_| LocalBuilderError::InvalidProject)
}

/// Returns the canonical supported JSON DSL after Rust validation.
#[uniffi::export]
pub fn canonical_local_json_dsl(source: String) -> Result<String, LocalBuilderError> {
    let project = from_dsl(&source, "local").map_err(|_| LocalBuilderError::InvalidProject)?;
    to_dsl(&project, "local").map_err(|_| LocalBuilderError::InvalidProject)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_project(tenant: &str) -> String {
        serde_json::json!({
            "schema_version": 1,
            "tenant_id": tenant,
            "project_id": "native_fixture",
            "revision": 1,
            "max_cost_minor": 0,
            "grants": [],
            "nodes": [
                {"id":"input","kind":"input","inputs":[],
                 "outputs":[{"name":"out","kind":"text"}],
                 "tool_name":null,"max_cost_minor":0},
                {"id":"output","kind":"output",
                 "inputs":[{"name":"in","kind":"text"}],"outputs":[],
                 "tool_name":null,"max_cost_minor":0}
            ],
            "edges":[{"from_node":"input","from_port":"out","to_node":"output","to_port":"in"}]
        }).to_string()
    }

    #[test]
    fn local_plan_is_deterministic_and_does_not_execute_effects() {
        let input = minimal_project("local");
        let first = validate_local_json_dsl(input.clone()).unwrap();
        let second = validate_local_json_dsl(input).unwrap();
        assert_eq!(first, second);
        let value: serde_json::Value = serde_json::from_str(&first).unwrap();
        assert_eq!(value["effects_executed"], false);
        assert_eq!(value["ordered_nodes"], serde_json::json!(["input", "output"]));
        assert_eq!(value["project_sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn rejected_cross_tenant_input_cannot_change_local_namespace() {
        assert!(validate_local_json_dsl(minimal_project("attacker")).is_err());
    }

    #[test]
    fn canonical_roundtrip_is_identical() {
        let sample = minimal_project("local");
        let first = canonical_local_json_dsl(sample).unwrap();
        let second = canonical_local_json_dsl(first.clone()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn unknown_critical_field_fails_closed() {
        let mut json: serde_json::Value = serde_json::from_str(&minimal_project("local")).unwrap();
        json["privileged_effect"] = serde_json::json!(true);
        assert!(validate_local_json_dsl(json.to_string()).is_err());
    }
}
