# FP&A Budgeting Agent (OCAF-AGENT-EXT-fpa-budgeting) — architecture

Role: Budget-versus-actual analysis, variance drivers and forecast support.

Native Rust package delegates bounded request validation, DGM/RSI proposal search and real Zenoh staging probes to common Rust core. No Python runtime, MCP or direct provider. OM Engine is the intended execution coordinator; OMAs admits identity, OMAt authorizes tools separately. Pinned source scope: STATIC_ROLE_LINE. Production ingress/model/OSTc/PQC not integrated. No active admission.
