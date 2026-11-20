# Ludonarrative Weaver (OCAF-AGENT-EXT-adult-ai-ludonarrative-weaver) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-ludonarrative-weaver/agent.py); blob 5ba7075622d72556e68410e1deec6a2bb1eecf75. Original ID: adult-ai-ludonarrative-weaver. Version 0.1.0.

Role: Stateful adult RPG and interactive-fiction design with explicit boundaries and reset controls.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
