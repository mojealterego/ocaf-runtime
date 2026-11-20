# Adult AI Voice Agent (OCAF-AGENT-EXT-adult-ai-voice-agent) — architecture

Role: Voice companion architecture, recording controls and synthetic-voice safeguards.

Native Rust package delegates bounded request validation, DGM/RSI proposal search and real Zenoh staging probes to common Rust core. No Python runtime, MCP or direct provider. OM Engine is the intended execution coordinator; OMAs admits identity, OMAt authorizes tools separately. Pinned source scope: REGISTRY_DESCRIPTION. Production ingress/model/OSTc/PQC not integrated. No active admission.
