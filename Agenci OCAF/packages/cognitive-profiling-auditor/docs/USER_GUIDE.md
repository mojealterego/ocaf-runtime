# Cognitive Profiling Auditor (OCAF-AGENT-EXT-cognitive-profiling-auditor) — user guide

Purpose: Audit of inferred-personality profiling.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-cognitive-profiling-auditor -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-cognitive-profiling-auditor -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
