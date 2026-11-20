# Medical Coding & Billing Agent (OCAF-AGENT-EXT-medical-coding) — user guide

Purpose: Documented coding support, evidence linkage and payer-rule uncertainty flags.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-medical-coding -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-medical-coding -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
