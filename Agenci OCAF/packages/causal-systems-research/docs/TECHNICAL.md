# Causal Systems Research Agent (OCAF-AGENT-EXT-causal-systems-research) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/causal-systems-research/agent.py); blob 3bb03589774eb3f3c8059440e66375de5f9046d4. Original ID: causal-systems-research. Version 0.1.0.

Role: Causal inference, identification audits and research validation.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
