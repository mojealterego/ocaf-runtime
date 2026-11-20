# Legacy Archivist (OCAF-AGENT-EXT-adult-ai-legacy-archivist) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-legacy-archivist/agent.py); blob 88bae3bc3063b03afc8dffded406654d48a8200f. Original ID: adult-ai-legacy-archivist. Version 0.1.0.

Role: Consent-controlled personal and family memory archiving with provenance, review and deletion controls.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
