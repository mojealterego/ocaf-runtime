# Agent Evaluation Ops (OCAF-AGENT-EXT-agent-evaluation-ops) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/agent-evaluation-ops/agent.py); blob b6e0c1823a6ae8f1a99dd66b9c5a63a8d8d15dbf. Original ID: agent-evaluation-ops. Version 0.1.0.

Role: AgentOps evaluation, golden datasets, judge calibration, drift detection and CI/CD circuit breakers.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
