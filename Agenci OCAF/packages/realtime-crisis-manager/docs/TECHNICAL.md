# Real-Time Crisis Manager (OCAF-AGENT-EXT-realtime-crisis-manager) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/realtime-crisis-manager/agent.py); blob 38d83e829b70d4070ee7adef9abf82cdd9490570. Original ID: realtime-crisis-manager. Version 0.1.0.

Role: Event-driven crisis architecture with low-latency deterministic controls and bounded model-based analysis.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
