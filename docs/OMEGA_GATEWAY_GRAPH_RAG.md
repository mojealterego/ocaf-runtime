# OMEGA Gateway GraphRAG core
Uses existing HybridIndex for BM25/cosine/RRF, atomic revision-bound snapshot
replacement, directed two-hop BFS, deterministic ranking, tenant binding,
source digests and bounded retrieval. Input is operator-admitted text, vectors
and explicit edges; no inferred relationships or embedding attestation.
No filesystem ingestion, extraction, persistent store, cloud transport or
automatic injection of retrieved text into model instructions is provided.
Native test command: cargo test --locked --test gateway_graph_rag.
Status: source implementation, native tests unexecuted.
