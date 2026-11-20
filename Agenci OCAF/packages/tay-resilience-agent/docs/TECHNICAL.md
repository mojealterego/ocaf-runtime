# Tay Resilience Agent (OCAF-AGENT-EXT-tay-resilience-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/tay-resilience-agent/agent.py); blob 407a639167c940ca5c55bce442783bfd39a47081. Original ID: tay-resilience-agent. Version 0.1.0.

Role: Continual-learning contamination and data-poisoning resilience using synthetic test corpora.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
