# Haptic Composer (OCAF-AGENT-EXT-adult-ai-haptic-composer) — user guide

Purpose: Declarative semantic-to-haptic planning separated from hardware actuation and bounded by safety policy.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-adult-ai-haptic-composer -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-adult-ai-haptic-composer -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
