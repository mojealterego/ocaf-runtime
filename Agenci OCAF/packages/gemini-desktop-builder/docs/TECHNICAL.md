# Gemini Desktop Builder (OCAF-AGENT-EXT-gemini-desktop-builder) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/gemini-desktop-builder/agent.py); blob fd243a5f1eb0945aa9e3e1ee93c625b65f1184e9. Original ID: gemini-desktop-builder. Version 0.1.0.

Role: Python/Google GenAI SDK/ADK desktop agent engineering.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
