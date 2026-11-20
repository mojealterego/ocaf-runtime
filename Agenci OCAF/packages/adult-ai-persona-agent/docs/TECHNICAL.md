# Adult AI Persona & Character Agent (OCAF-AGENT-EXT-adult-ai-persona-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-persona-agent/agent.py); blob d6831b4dd02b4dc901e1b9aacb3c85490da57a8d. Original ID: adult-ai-persona-agent. Version 0.1.0.

Role: Versioned fictional personas, character canon and anti-impersonation controls.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
