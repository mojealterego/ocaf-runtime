# Gemini Edge RAG Agent (OCAF-AGENT-EXT-gemini-edge-rag) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/gemini-edge-rag/agent.py); blob 70bee1afeaac63001d7b5e010bccd063967309a2. Original ID: gemini-edge-rag. Version 0.1.0.

Role: On-device RAG, provenance, local indexing and cloud fallback controls.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
