# Child AI Content Moderation Agent (OCAF-AGENT-EXT-child-ai-content-moderation-agent) — user guide

Purpose: Age-banded content classification, filtering and escalation.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-child-ai-content-moderation-agent -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-child-ai-content-moderation-agent -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
