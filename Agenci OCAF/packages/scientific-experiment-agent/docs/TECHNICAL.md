# Scientific Experiment Agent (OCAF-AGENT-EXT-scientific-experiment-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/scientific-experiment-agent/agent.py); blob 3b5504cbb67284d717b420231496cb568f1734bd. Original ID: scientific-experiment-agent. Version 0.1.0.

Role: Scientific experiment design, hardware-in-the-loop orchestration, data integrity and reproducible analysis.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
