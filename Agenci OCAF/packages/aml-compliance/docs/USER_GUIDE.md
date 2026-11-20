# AML Compliance Review Agent (OCAF-AGENT-EXT-aml-compliance) — user guide

Purpose: Transaction-alert triage, evidence mapping and compliance escalation drafting.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-aml-compliance -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-aml-compliance -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
