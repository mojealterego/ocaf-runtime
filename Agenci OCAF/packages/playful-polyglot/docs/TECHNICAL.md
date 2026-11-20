# Playful Polyglot (OCAF-AGENT-EXT-playful-polyglot) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/school-playful-polyglot/agent.py); blob 8fbe334181dc648db12b8518f12b1511c80638c8. Original ID: playful-polyglot. Version 0.1.0.

Role: Playful foreign-language learning for grades I–III.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
