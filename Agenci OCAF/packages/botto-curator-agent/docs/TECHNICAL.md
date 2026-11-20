# Botto Curator Agent (OCAF-AGENT-EXT-botto-curator-agent) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/botto-curator-agent/agent.py); blob 871f03d441174215882d2d01df3dd2b6d2b9f813. Original ID: botto-curator-agent. Version 0.1.0.

Role: Simulation of generative-art curation, community taste feedback and virtual treasury.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
