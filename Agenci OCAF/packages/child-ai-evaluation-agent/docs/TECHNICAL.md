# Child AI Evaluation Agent (OCAF-AGENT-EXT-child-ai-evaluation-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/child-ai-evaluation-agent/agent.py); blob 05738742b716d67e8317703d8c28ac678825b6d3. Original ID: child-ai-evaluation-agent. Version 0.1.0.

Role: Safety, quality, privacy and regression evaluation for child AI.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
