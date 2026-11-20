# Regulatory Watchdog Agent (OCAF-AGENT-EXT-regulatory-watch) — user guide

Purpose: Official-source regulatory change tracking, impact mapping and dated alerts.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-regulatory-watch -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-regulatory-watch -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
