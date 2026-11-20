# Creative Writing Room (OCAF-AGENT-EXT-creative-writing-room) — user guide

Purpose: Long-form fiction planning, world-bible, bounded drafting, RAG and continuity control.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-creative-writing-room -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-creative-writing-room -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
