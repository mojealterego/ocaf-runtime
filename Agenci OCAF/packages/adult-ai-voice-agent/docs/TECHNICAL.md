# Adult AI Voice Agent (OCAF-AGENT-EXT-adult-ai-voice-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-voice-agent/agent.py); blob ce93c27302963dba2a4427fba6d8ee1c7d15a209. Original ID: adult-ai-voice-agent. Version 0.1.0.

Role: Voice companion architecture, recording controls and synthetic-voice safeguards.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
