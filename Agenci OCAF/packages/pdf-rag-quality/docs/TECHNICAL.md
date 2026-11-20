# PDF Extraction & RAG Quality Agent (OCAF-AGENT-EXT-pdf-rag-quality) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/pdf-rag-quality/agent.py); blob e2a80e0d839d8e3fda4d14789266daf2b64d439e. Original ID: pdf-rag-quality. Version 0.1.0.

Role: PDF extraction, provenance and RAG evaluation.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
