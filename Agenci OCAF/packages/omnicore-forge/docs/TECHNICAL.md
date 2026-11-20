# OmniCore Forge (OCAF-AGENT-EXT-omnicore-forge) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/omnicore-forge/agent.py); blob f71a640c230270717dcde4192046170900c52053. Original ID: omnicore-forge. Version 0.1.0.

Role: Private AI infrastructure, nested virtualization, GPU/RAG and Rust kernel engineering with safety gates.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
