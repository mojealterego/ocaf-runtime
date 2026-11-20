# Regulatory Watchdog Agent (OCAF-AGENT-EXT-regulatory-watch) — architecture

Role: Official-source regulatory change tracking, impact mapping and dated alerts.

Native Rust package delegates bounded request validation, DGM/RSI proposal search and real Zenoh staging probes to common Rust core. No Python runtime, MCP or direct provider. OM Engine is the intended execution coordinator; OMAs admits identity, OMAt authorizes tools separately. Pinned source scope: STATIC_ROLE_LINE. Production ingress/model/OSTc/PQC not integrated. No active admission.
