# Gemini Security Auditor (OCAF-AGENT-EXT-gemini-security-auditor) — user guide

Purpose: Security assessment for Gemini builders, tools, MCP and data boundaries.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-gemini-security-auditor -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-gemini-security-auditor -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
