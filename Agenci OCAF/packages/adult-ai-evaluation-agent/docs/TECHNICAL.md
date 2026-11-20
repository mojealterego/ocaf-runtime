# Adult AI Companion Evaluation Agent (OCAF-AGENT-EXT-adult-ai-evaluation-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-evaluation-agent/agent.py); blob fa9c9494966b2dc5a1ef9dfcde8266ea05f19d32. Original ID: adult-ai-evaluation-agent. Version 0.1.0.

Role: Quality, safety, consent, privacy and operations evaluation.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
