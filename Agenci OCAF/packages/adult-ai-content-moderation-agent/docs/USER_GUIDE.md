# Adult AI Content Moderation Agent (OCAF-AGENT-EXT-adult-ai-content-moderation-agent) — user guide

Purpose: Policy-aware moderation and hard boundaries for adult platforms.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-adult-ai-content-moderation-agent -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-adult-ai-content-moderation-agent -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
