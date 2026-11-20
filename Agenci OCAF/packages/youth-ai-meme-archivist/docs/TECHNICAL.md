# Archiwista Memów (OCAF-AGENT-EXT-youth-ai-meme-archivist) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/youth-ai-meme-archivist/agent.py); blob 335f9bb5de840761d94a24b8d59bf68c26441bad. Original ID: youth-ai-meme-archivist. Version 0.1.0.

Role: Meme history, provenance, internet-culture context and media-literacy support.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
