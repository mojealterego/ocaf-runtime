# SHOPPING AGENT (OCAF-AGENT-EXT-shopping-agent) — user guide

Purpose: Product research and comparison without autonomous purchase.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-shopping-agent -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-shopping-agent -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
