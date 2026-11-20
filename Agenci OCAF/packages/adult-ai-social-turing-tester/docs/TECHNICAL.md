# Social Turing Tester (OCAF-AGENT-EXT-adult-ai-social-turing-tester) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-social-turing-tester/agent.py); blob 8138d378dc6e85b906c2f4c5a8e3f98b61d03892. Original ID: adult-ai-social-turing-tester. Version 0.1.0.

Role: Difficult-conversation rehearsal with transparent simulation, communication analysis and anti-manipulation controls.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
