# Gemini Edge RAG Agent (OCAF-AGENT-EXT-gemini-edge-rag) — user guide

Purpose: On-device RAG, provenance, local indexing and cloud fallback controls.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-gemini-edge-rag -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-gemini-edge-rag -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
