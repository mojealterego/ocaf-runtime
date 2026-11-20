# Stylista Cyfrowy (OCAF-AGENT-EXT-youth-ai-digital-stylist) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/youth-ai-digital-stylist/agent.py); blob b254c220a34340a15423b930a4283c4396c260a1. Original ID: youth-ai-digital-stylist. Version 0.1.0.

Role: Personal style, wardrobe planning and sustainable choices without body or attractiveness ratings.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
