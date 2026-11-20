# Persuasion & Dark-Patterns Auditor (OCAF-AGENT-EXT-persuasion-dark-patterns-auditor) — user guide

Purpose: Detection of manipulative UX patterns.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-persuasion-dark-patterns-auditor -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-persuasion-dark-patterns-auditor -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
