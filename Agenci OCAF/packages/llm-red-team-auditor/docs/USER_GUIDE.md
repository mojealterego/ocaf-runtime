# LLM Red-Team Auditor (OCAF-AGENT-EXT-llm-red-team-auditor) — user guide

Purpose: Authorized LLM security assessment.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-llm-red-team-auditor -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-llm-red-team-auditor -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
