# Cyrano — Social Vetting (OCAF-AGENT-EXT-adult-ai-cyrano-social-vetting) — user guide

Purpose: Evidence-first social vetting without face-based age inference, diagnosis or covert surveillance.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-adult-ai-cyrano-social-vetting -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-adult-ai-cyrano-social-vetting -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
