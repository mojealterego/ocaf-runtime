# ChaosGPT Safety Simulator (OCAF-AGENT-EXT-chaosgpt-safety-agent) — architecture

Role: Defensive sandbox for goal-misalignment, tool escalation and containment testing.

Native Rust package delegates bounded request validation, DGM/RSI proposal search and real Zenoh staging probes to common Rust core. No Python runtime, MCP or direct provider. OM Engine is the intended execution coordinator; OMAs admits identity, OMAt authorizes tools separately. Pinned source scope: REGISTRY_DESCRIPTION. Production ingress/model/OSTc/PQC not integrated. No active admission.
