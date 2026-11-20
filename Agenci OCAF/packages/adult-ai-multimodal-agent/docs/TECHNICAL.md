# Adult AI Multimodal Companion Agent (OCAF-AGENT-EXT-adult-ai-multimodal-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-multimodal-agent/agent.py); blob 2b3157f45f69750a561e35d174538af3386cf6bd. Original ID: adult-ai-multimodal-agent. Version 0.1.0.

Role: Consent-aware orchestration of text, voice, image and video.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
