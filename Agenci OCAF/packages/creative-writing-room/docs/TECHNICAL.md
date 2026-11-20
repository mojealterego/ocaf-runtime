# Creative Writing Room (OCAF-AGENT-EXT-creative-writing-room) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/creative-writing-room/agent.py); blob 2bf9ef2475be37f2966b29d6dc8cb41b7692ec54. Original ID: creative-writing-room. Version 0.1.0.

Role: Long-form fiction planning, world-bible, bounded drafting, RAG and continuity control.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
