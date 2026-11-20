# Child AI Content Moderation Agent (OCAF-AGENT-EXT-child-ai-content-moderation-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/child-ai-content-moderation-agent/agent.py); blob de392cdcffe5dbba147f71605fa6ae59fa3f1ae0. Original ID: child-ai-content-moderation-agent. Version 0.1.0.

Role: Age-banded content classification, filtering and escalation.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
