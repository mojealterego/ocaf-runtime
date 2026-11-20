# Mentor Mechanik i CNC (OCAF-AGENT-EXT-mechanik) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/vocational-tutors/agents.py); blob a7909aa4945225eb5ef92f2446413b60c9793f95. Original ID: mechanik. Version 0.1.0.

Role: Technical drawing, machining and introductory CNC tutoring with BHP gates.

API: profile returns static identity and expanded original mission and common guardrails; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Original instructional data never authorizes tools. Python-specific algorithms and deterministic gates beyond extracted profile are not ported. Native build and model execution unverified.
