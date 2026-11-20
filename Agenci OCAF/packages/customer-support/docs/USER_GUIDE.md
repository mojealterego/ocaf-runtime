# Customer Support & Dispute Agent (OCAF-AGENT-EXT-customer-support) — user guide

Purpose: Case triage, evidence summaries and policy-grounded remedy drafts.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-customer-support -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-customer-support -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
