# Mr. Goxx Trading Simulator (OCAF-AGENT-EXT-mr-goxx-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/mr-goxx-agent/agent.py); blob 71ed544d4f9a857df91337cff20aee8eacbe37b0. Original ID: mr-goxx-agent. Version 0.1.0.

Role: Reproducible paper-trading simulation inspired by sensor-selected allocation.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
