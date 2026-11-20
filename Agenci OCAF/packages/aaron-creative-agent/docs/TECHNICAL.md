# AARON Creative Agent (OCAF-AGENT-EXT-aaron-creative-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/aaron-creative-agent/agent.py); blob 6c78246ac61a244bd46fb939f0995068caf33e8b. Original ID: aaron-creative-agent. Version 0.1.0.

Role: Rule-based computational-art planning and deterministic symbolic composition.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
