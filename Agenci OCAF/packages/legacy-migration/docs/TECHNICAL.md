# Legacy Code Migration Agent (OCAF-AGENT-EXT-legacy-migration) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/occupational-ai/agent.py); blob 507a659a1d813e15acd725704343605e900146cf. Original ID: legacy-migration. Version 0.1.0.

Role: Behavior-preserving legacy migration planning, tests and compatibility analysis.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
