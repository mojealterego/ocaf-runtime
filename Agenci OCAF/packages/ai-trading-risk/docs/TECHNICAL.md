# AI Trading Risk Agent (OCAF-AGENT-EXT-ai-trading-risk) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/portfolio_agent.py); blob e556ffdeb6a4e1124a41a3c82d770d70dbd4716a. Original ID: ai-trading-risk. Version 0.1.0.

Role: Trading risk analysis without trade execution.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
