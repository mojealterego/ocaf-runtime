# Adult AI Companion Architect (OCAF-AGENT-EXT-adult-ai-companion-architect) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-companion-architect/agent.py); blob 7d52c305c7fd90b1fae80e0677880556fbd9c5a1. Original ID: adult-ai-companion-architect. Version 0.1.0.

Role: Architecture, privacy, consent, safety and evaluation for adult-oriented AI companion software.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
