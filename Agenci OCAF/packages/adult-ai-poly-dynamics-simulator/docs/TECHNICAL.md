# Poly-Dynamics Simulator (OCAF-AGENT-EXT-adult-ai-poly-dynamics-simulator) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-poly-dynamics-simulator/agent.py); blob 90ca292c634a2354b9dfa76791b191c05ba5c9ca. Original ID: adult-ai-poly-dynamics-simulator. Version 0.1.0.

Role: Fictional multi-adult relationship simulation with explicit per-person consent and state.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
