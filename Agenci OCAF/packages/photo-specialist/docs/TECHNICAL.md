# Photo Specialist (OCAF-AGENT-EXT-photo-specialist) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/photo/specialist_agent.py); blob 19cb4760cb1b60f10ba551a172fa068c94de2a87. Original ID: photo-specialist. Version 0.1.0.

Role: Profile-driven photographic prompt generation.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
