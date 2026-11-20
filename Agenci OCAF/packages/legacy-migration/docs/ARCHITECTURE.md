# Legacy Code Migration Agent (OCAF-AGENT-EXT-legacy-migration) — architecture

Role: Behavior-preserving legacy migration planning, tests and compatibility analysis.

Native Rust package delegates bounded request validation, DGM/RSI proposal search and real Zenoh staging probes to common Rust core. No Python runtime, MCP or direct provider. OM Engine is the intended execution coordinator; OMAs admits identity, OMAt authorizes tools separately. Pinned source scope: STATIC_EXPANDED_ROLE. Production ingress/model/OSTc/PQC not integrated. No active admission.
