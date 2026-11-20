# Child AI Privacy Agent (OCAF-AGENT-EXT-child-ai-privacy-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/child-ai-privacy-agent/agent.py); blob 812c05aa3e022e04b707705758cac8c04259e883. Original ID: child-ai-privacy-agent. Version 0.1.0.

Role: Data-flow, minimization, retention, deletion and processor governance.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
