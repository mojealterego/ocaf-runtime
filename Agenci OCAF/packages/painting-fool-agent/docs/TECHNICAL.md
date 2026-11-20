# Painting Fool Agent (OCAF-AGENT-EXT-painting-fool-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/painting-fool-agent/agent.py); blob 1897557e7482dc1c9c3ee7e9bef149566256102c. Original ID: painting-fool-agent. Version 0.1.0.

Role: State-driven computational creativity with explainable simulated mood conditions.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
