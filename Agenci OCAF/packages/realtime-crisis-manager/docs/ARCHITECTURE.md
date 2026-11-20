# Real-Time Crisis Manager (OCAF-AGENT-EXT-realtime-crisis-manager) — architecture

Role: Event-driven crisis architecture with low-latency deterministic controls and bounded model-based analysis.

Native Rust package delegates bounded request validation, DGM/RSI proposal search and real Zenoh staging probes to common Rust core. No Python runtime, MCP or direct provider. OM Engine is the intended execution coordinator; OMAs admits identity, OMAt authorizes tools separately. Pinned source scope: REGISTRY_DESCRIPTION. Production ingress/model/OSTc/PQC not integrated. No active admission.
