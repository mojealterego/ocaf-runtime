# AI Talent Scout & Recruiter (OCAF-AGENT-EXT-talent-recruiter) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/occupational-ai/agent.py); blob 507a659a1d813e15acd725704343605e900146cf. Original ID: talent-recruiter. Version 0.1.0.

Role: Job-relevant candidate evidence mapping and recruiting workflow support.

API: profile returns static identity and expanded original mission and common guardrails; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Original instructional data never authorizes tools. Python-specific algorithms and deterministic gates beyond extracted profile are not ported. Native build and model execution unverified.
