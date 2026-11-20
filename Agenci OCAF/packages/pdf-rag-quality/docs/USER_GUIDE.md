# PDF Extraction & RAG Quality Agent (OCAF-AGENT-EXT-pdf-rag-quality) — user guide

Purpose: PDF extraction, provenance and RAG evaluation.

Run from repo root: `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-pdf-rag-quality -- profile` or `cargo run --manifest-path "Agenci OCAF/Cargo.toml" -p ocaf-ext-pdf-rag-quality -- prepare tenant_a req_1 < task.txt`. Output is an advisory draft, not a model response. Use authorized tenant data only. Agent Chat and OM Engine dispatch remain unavailable.
