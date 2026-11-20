# Agent Supervisor & Kill Switch (OCAF-AGENT-EXT-agent-supervisor-killswitch) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/agent-supervisor-killswitch/agent.py); blob 1de896417515dc67a73767b7a193a39a4c83bafe. Original ID: agent-supervisor-killswitch. Version 0.1.0.

Role: External policy proxy, hard budgets, anomaly detection, kill switch and bounded recovery for autonomous agents.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
