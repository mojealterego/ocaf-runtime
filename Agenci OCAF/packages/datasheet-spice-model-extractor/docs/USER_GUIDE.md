# Datasheet-to-SPICE Model Agent (OCAF-AGENT-EXT-datasheet-spice-model-extractor) — user guide

Purpose: Datasheet parameter extraction and candidate SPICE validation.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-datasheet-spice-model-extractor -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-datasheet-spice-model-extractor -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
