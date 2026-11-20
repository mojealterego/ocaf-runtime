# Cross-SaaS Orchestrator (OCAF-AGENT-EXT-cross-saas-orchestrator) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/cross-saas-orchestrator/agent.py); blob 9ac825027f2e182152488be477a034c93cfff96d. Original ID: cross-saas-orchestrator. Version 0.1.0.

Role: Provider-neutral cross-SaaS workflow orchestration, MCP-style tool gating, identity scopes and compensation.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
