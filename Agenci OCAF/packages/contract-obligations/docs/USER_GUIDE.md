# Contract Obligation Agent (OCAF-AGENT-EXT-contract-obligations) — user guide

Purpose: Contract obligations, deadlines and renewal extraction.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-contract-obligations -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-contract-obligations -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
