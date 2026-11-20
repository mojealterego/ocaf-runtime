# Adult AI Safety Agent (OCAF-AGENT-EXT-adult-ai-safety-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-safety-agent/agent.py); blob 2eb40ba44767bd21375ccf65b00fcf0ac81c1b1f. Original ID: adult-ai-safety-agent. Version 0.1.0.

Role: Adult-only companion safety, consent boundaries and abuse resistance.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
