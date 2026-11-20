# Geo-Strateg (OCAF-AGENT-EXT-geo-strateg) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/school-geo-strateg/agent.py); blob c45b88869c288fb4524328b92aa23b3984eb3b8b. Original ID: geo-strateg. Version 0.1.0.

Role: Geography for grades V–VIII.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
