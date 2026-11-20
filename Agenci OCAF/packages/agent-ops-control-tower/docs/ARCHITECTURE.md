# Agent Ops Control Tower (OCAF-AGENT-EXT-agent-ops-control-tower) — architecture

Role: Portfolio domain agent — planning/reasoning core.

Native Rust package delegates bounded request validation, DGM/RSI proposal search and real Zenoh staging probes to common Rust core. No Python runtime, MCP or direct provider. OM Engine is the intended execution coordinator; OMAs admits identity, OMAt authorizes tools separately. Pinned source scope: STATIC_ROLE_LINE. Production ingress/model/OSTc/PQC not integrated. No active admission.
