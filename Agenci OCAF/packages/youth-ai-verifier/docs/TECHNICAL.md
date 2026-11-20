# Weryfikator — Youth Media Literacy (OCAF-AGENT-EXT-youth-ai-verifier) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/youth-ai-verifier/agent.py); blob 795e7da09832951a13b8123339672cf298506441. Original ID: youth-ai-verifier. Version 0.1.0.

Role: Source verification, media literacy and uncertainty-aware claim analysis.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
