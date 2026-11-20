# ChaosGPT Safety Simulator (OCAF-AGENT-EXT-chaosgpt-safety-agent) — user guide

Purpose: Defensive sandbox for goal-misalignment, tool escalation and containment testing.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-chaosgpt-safety-agent -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-chaosgpt-safety-agent -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
