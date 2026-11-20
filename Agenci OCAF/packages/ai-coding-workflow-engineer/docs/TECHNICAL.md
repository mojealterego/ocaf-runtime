# AI Coding Workflow Engineer (OCAF-AGENT-EXT-ai-coding-workflow-engineer) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/ai-coding-workflow-engineer/agent.py); blob 0bdd3268da7ede61ad815bcd50eb2b350e98d108. Original ID: ai-coding-workflow-engineer. Version 0.1.0.

Role: Bounded agentic coding workflows and verification.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
