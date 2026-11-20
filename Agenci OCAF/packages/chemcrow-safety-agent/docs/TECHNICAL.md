# ChemCrow Safety Agent (OCAF-AGENT-EXT-chemcrow-safety-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/chemcrow-safety-agent/agent.py); blob 9f705722b0fd34a1593daf38756eb845994ad173. Original ID: chemcrow-safety-agent. Version 0.1.0.

Role: Chemistry literature and computational assistance with strict safety boundaries.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
