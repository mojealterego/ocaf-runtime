# Gemini Security Auditor (OCAF-AGENT-EXT-gemini-security-auditor) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/gemini-security-auditor/agent.py); blob ad922abf45f6fe77e4ba0aff58dd97f3e21d1913. Original ID: gemini-security-auditor. Version 0.1.0.

Role: Security assessment for Gemini builders, tools, MCP and data boundaries.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
