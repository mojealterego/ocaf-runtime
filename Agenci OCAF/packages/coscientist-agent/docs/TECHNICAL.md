# Coscientist Research Agent (OCAF-AGENT-EXT-coscientist-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/coscientist-agent/agent.py); blob f5f18331320835cebaee44c311cb8473c2d328d1. Original ID: coscientist-agent. Version 0.1.0.

Role: Scientific hypothesis, experiment-design and reproducibility planning without autonomous lab execution.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
