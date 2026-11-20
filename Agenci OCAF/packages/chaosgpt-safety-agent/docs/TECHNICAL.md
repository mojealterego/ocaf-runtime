# ChaosGPT Safety Simulator (OCAF-AGENT-EXT-chaosgpt-safety-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/chaosgpt-safety-agent/agent.py); blob 546aaaf0309153054d7262b899c475b3a23ce25f. Original ID: chaosgpt-safety-agent. Version 0.1.0.

Role: Defensive sandbox for goal-misalignment, tool escalation and containment testing.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
