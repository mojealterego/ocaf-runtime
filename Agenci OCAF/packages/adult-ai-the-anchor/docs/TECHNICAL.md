# The Anchor (OCAF-AGENT-EXT-adult-ai-the-anchor) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-the-anchor/agent.py); blob c07b62a15dae9f21912226a8779e655bef7a4b1e. Original ID: adult-ai-the-anchor. Version 0.1.0.

Role: Non-clinical grounding and self-regulation architecture with crisis and human-handoff boundaries.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
