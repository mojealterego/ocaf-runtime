# AI Steve Civic Agent (OCAF-AGENT-EXT-ai-steve-civic-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/ai-steve-civic-agent/agent.py); blob 903324d76b3c5d02bc3646cd75bd56190bbe3b86. Original ID: ai-steve-civic-agent. Version 0.1.0.

Role: Neutral civic aggregation and deliberation simulator without voter targeting or persuasion.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
