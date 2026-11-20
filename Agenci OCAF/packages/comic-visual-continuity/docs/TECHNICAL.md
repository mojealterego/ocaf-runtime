# Comic Visual Continuity Agent (OCAF-AGENT-EXT-comic-visual-continuity) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/comic-visual-continuity/agent.py); blob 8f825f8fe6ab232a6c6197c63ab4576eda0e8db3. Original ID: comic-visual-continuity. Version 0.1.0.

Role: Comic panel scripting, character/reference consistency and visual continuity QA.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
