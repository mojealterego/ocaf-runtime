# Photo Agent (OCAF-AGENT-EXT-photo-general) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/photo/agent.py); blob 5076e97c81b7b9df81e3ce94522ce106b769a2ac. Original ID: photo-general. Version 0.1.0.

Role: Documentary photography, visual concepts, and photojournalism assistance.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
