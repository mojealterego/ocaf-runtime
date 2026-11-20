# Adult AI Proactive Messaging Agent (OCAF-AGENT-EXT-adult-ai-proactive-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-proactive-agent/agent.py); blob 7b56b3672ae3bc73f2c735efcec6edf63413377d. Original ID: adult-ai-proactive-agent. Version 0.1.0.

Role: Opt-in proactive messaging, quiet hours, revocation and anti-dependency safeguards.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
