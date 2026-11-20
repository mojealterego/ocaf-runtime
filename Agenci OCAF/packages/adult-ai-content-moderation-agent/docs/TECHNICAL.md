# Adult AI Content Moderation Agent (OCAF-AGENT-EXT-adult-ai-content-moderation-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-content-moderation-agent/agent.py); blob 5e8b7b7afbf6cf1f81f5eca65a83faeb58a24c71. Original ID: adult-ai-content-moderation-agent. Version 0.1.0.

Role: Policy-aware moderation and hard boundaries for adult platforms.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
