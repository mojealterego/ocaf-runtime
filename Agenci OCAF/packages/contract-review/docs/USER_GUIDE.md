# Contract Review & Redlining Agent (OCAF-AGENT-EXT-contract-review) — user guide

Purpose: Clause evidence extraction, playbook comparison and redline drafting.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-contract-review -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-contract-review -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
