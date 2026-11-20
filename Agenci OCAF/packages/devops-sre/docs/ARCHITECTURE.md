# Autonomous DevOps / SRE Agent (OCAF-AGENT-EXT-devops-sre) — architecture

Role: Alert triage, incident correlation and bounded remediation proposals.

Native Rust package delegates bounded request validation, DGM/RSI proposal search and real Zenoh staging probes to common Rust core. No Python runtime, MCP or direct provider. OM Engine is the intended execution coordinator; OMAs admits identity, OMAt authorizes tools separately. Pinned source scope: STATIC_ROLE_LINE. Production ingress/model/OSTc/PQC not integrated. No active admission.
