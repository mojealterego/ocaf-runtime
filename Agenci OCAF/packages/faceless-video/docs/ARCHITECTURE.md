# Faceless Video Agent (OCAF-AGENT-EXT-faceless-video) — architecture

Role: Faceless video production planning.

Native Rust package delegates bounded request validation, DGM/RSI proposal search and real Zenoh staging probes to common Rust core. No Python runtime, MCP or direct provider. OM Engine is the intended execution coordinator; OMAs admits identity, OMAt authorizes tools separately. Pinned source scope: STATIC_EXPANDED_ROLE. Production ingress/model/OSTc/PQC not integrated. No active admission.
