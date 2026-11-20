# Filozof Moralny (OCAF-AGENT-EXT-filozof-moralny) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/school-filozof-moralny/agent.py); blob e36b53a37fe2af0e265c815e6f17b33b4a05a4aa. Original ID: filozof-moralny. Version 0.1.0.

Role: Ethics and philosophical reasoning for school learners.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
