# Child AI Evaluation Agent (OCAF-AGENT-EXT-child-ai-evaluation-agent) — user guide

Purpose: Safety, quality, privacy and regression evaluation for child AI.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-child-ai-evaluation-agent -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-child-ai-evaluation-agent -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
