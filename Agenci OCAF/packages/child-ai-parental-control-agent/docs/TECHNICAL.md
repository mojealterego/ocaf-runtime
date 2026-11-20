# Child AI Parental Control Agent (OCAF-AGENT-EXT-child-ai-parental-control-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/child-ai-parental-control-agent/agent.py); blob ddc749fa89aed7a64d5d1772b4a33bff95fe8335. Original ID: child-ai-parental-control-agent. Version 0.1.0.

Role: Guardian consent, permissions, feature controls and revocation.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
