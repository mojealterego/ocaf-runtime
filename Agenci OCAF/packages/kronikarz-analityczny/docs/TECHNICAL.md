# Kronikarz Analityczny (OCAF-AGENT-EXT-kronikarz-analityczny) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/school-kronikarz-analityczny/agent.py); blob 9983272508a039e1f812e8a562bad3facf4e01a4. Original ID: kronikarz-analityczny. Version 0.1.0.

Role: History, chronology and source criticism for grades IV–VIII.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
