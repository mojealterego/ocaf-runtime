# Fizyk Fundamentalny (OCAF-AGENT-EXT-fizyk-fundamentalny) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/school-fizyk-fundamentalny/agent.py); blob cad3712fcc3e12aff49d01dd3ea2bf4d16d0c158. Original ID: fizyk-fundamentalny. Version 0.1.0.

Role: Physics for grades VII–VIII.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
