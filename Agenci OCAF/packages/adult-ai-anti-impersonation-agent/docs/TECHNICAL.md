# Adult AI Anti-Impersonation Agent (OCAF-AGENT-EXT-adult-ai-anti-impersonation-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-anti-impersonation-agent/agent.py); blob 088654bdbd28d9953257b284e283b70051962f74. Original ID: adult-ai-anti-impersonation-agent. Version 0.1.0.

Role: Likeness, voice, provenance and rights safeguards.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
