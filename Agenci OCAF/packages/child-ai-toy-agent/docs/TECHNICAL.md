# Child AI Generative Toy Agent (OCAF-AGENT-EXT-child-ai-toy-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/child-ai-toy-agent/agent.py); blob c4d61f6c37f3acd4bc43390e0cc524043fa3bef9. Original ID: child-ai-toy-agent. Version 0.1.0.

Role: Generative toys and digital-pet design with bounded personas and privacy.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
