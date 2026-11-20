# Weryfikator — Youth Media Literacy (OCAF-AGENT-EXT-youth-ai-verifier) — user guide

Purpose: Source verification, media literacy and uncertainty-aware claim analysis.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-youth-ai-verifier -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-youth-ai-verifier -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
