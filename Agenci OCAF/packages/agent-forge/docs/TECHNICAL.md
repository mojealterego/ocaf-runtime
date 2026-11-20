# Agent Forge (OCAF-AGENT-EXT-agent-forge) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/agent-forge/agent.py); blob 512963e2d072a780f06f05b99367a51c19b44c8b. Original ID: agent-forge. Version 0.1.0.

Role: Compiles research into agent capabilities, upgrades, tests and verification gates.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
