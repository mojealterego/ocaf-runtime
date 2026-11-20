# Autonomous Procurement Agent (OCAF-AGENT-EXT-procurement) — user guide

Purpose: Supplier comparison, sourcing analysis and purchase-order proposal drafting.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-procurement -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-procurement -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
