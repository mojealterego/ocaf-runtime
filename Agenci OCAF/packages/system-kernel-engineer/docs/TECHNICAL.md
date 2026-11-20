# System Kernel Engineer (OCAF-AGENT-EXT-system-kernel-engineer) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/system-kernel-engineer/agent.py); blob 35962972d6ceafe6dfa2642cfa9bc5da114ccb3c. Original ID: system-kernel-engineer. Version 0.1.0.

Role: Low-level OS/kernel, Rust/no_std, C-to-Rust, GPU and hardware-interface engineering with verification gates.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
