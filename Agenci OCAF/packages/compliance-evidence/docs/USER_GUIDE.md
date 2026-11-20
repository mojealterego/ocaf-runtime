# Compliance Evidence Agent (OCAF-AGENT-EXT-compliance-evidence) — user guide

Purpose: Collect, map and verify audit evidence.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-compliance-evidence -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-compliance-evidence -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
