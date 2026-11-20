# Mistrz Logiki (OCAF-AGENT-EXT-mistrz-logiki) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/school-mistrz-logiki/agent.py); blob c270104eb34af6c8d090800710e560e2beb1cd26. Original ID: mistrz-logiki. Version 0.1.0.

Role: Mathematics for grades IV–VIII with error diagnosis and modeling.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
