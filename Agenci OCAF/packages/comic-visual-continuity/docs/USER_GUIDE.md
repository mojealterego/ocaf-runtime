# Comic Visual Continuity Agent (OCAF-AGENT-EXT-comic-visual-continuity) — user guide

Purpose: Comic panel scripting, character/reference consistency and visual continuity QA.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-comic-visual-continuity -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-comic-visual-continuity -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
