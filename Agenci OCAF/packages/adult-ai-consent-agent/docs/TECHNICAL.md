# Adult AI Consent & Boundaries Agent (OCAF-AGENT-EXT-adult-ai-consent-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-consent-agent/agent.py); blob 29cf56d38fab687b380a0de350c649cbd12f6283. Original ID: adult-ai-consent-agent. Version 0.1.0.

Role: Explicit consent, revocation and scoped permission architecture.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
