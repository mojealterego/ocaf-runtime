# Quality Diversity Engineer (OCAF-AGENT-EXT-quality-diversity-engineer) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/quality-diversity-engineer/agent.py); blob 17ee31741e53cc9c81183e2cbce8bbbfb8c97b77. Original ID: quality-diversity-engineer. Version 0.1.0.

Role: MAP-Elites, Novelty Search, evolutionary archives and reproducible quality-diversity experiments.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
