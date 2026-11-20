# Organizator Imprez (OCAF-AGENT-EXT-youth-ai-safe-party-planner) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/youth-ai-safe-party-planner/agent.py); blob 55383d29de4b2fb08e98cf5aab064f49a4d9259a. Original ID: youth-ai-safe-party-planner. Version 0.1.0.

Role: Age-appropriate, substance-free event planning with safety, accessibility and return-home safeguards.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
