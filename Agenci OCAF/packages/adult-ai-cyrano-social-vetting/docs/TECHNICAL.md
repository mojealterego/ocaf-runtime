# Cyrano — Social Vetting (OCAF-AGENT-EXT-adult-ai-cyrano-social-vetting) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-cyrano-social-vetting/agent.py); blob ebef174a418f6fbe04f88373a045499b5bebe9f6. Original ID: adult-ai-cyrano-social-vetting. Version 0.1.0.

Role: Evidence-first social vetting without face-based age inference, diagnosis or covert surveillance.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
