# Kustosz Słowa (OCAF-AGENT-EXT-kustosz-slowa) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/school-kustosz-slowa/agent.py); blob cafc3ddca1e5fe6db716aaeb75676f84cb218634. Original ID: kustosz-slowa. Version 0.1.0.

Role: Polish language, literature and writing for grades IV–VIII.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
