# Coscientist Research Agent (OCAF-AGENT-EXT-coscientist-agent) — user guide

Purpose: Scientific hypothesis, experiment-design and reproducibility planning without autonomous lab execution.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-coscientist-agent -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-coscientist-agent -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
