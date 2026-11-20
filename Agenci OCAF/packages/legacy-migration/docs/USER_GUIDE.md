# Legacy Code Migration Agent (OCAF-AGENT-EXT-legacy-migration) — user guide

Purpose: Behavior-preserving legacy migration planning, tests and compatibility analysis.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-legacy-migration -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-legacy-migration -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
