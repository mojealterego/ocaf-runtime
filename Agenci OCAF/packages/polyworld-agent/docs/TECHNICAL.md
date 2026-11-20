# Polyworld Agent (OCAF-AGENT-EXT-polyworld-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/polyworld-agent/agent.py); blob 09253c94092535bd36903d8fb08218fad0769e85. Original ID: polyworld-agent. Version 0.1.0.

Role: Artificial-life simulation with evolving digital organisms, genomes and neural controllers.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
