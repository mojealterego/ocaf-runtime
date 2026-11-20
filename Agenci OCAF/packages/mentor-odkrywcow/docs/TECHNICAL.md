# Mentor Odkrywców (OCAF-AGENT-EXT-mentor-odkrywcow) — technical specification

Source: [pinned entrypoint](https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/school-mentor-odkrywcow/agent.py); blob 564ce1b8f0451e5e0f6a52baa8de9a5eee701b7b. Original ID: mentor-odkrywcow. Version 0.1.0.

Role: Integrated primary-school learning for grades I–III.

API: profile returns static identity and role; prepare produces a draft with DENY, zero tools, four-step budget and 65,536-byte limit. Zenoh probe uses actual correlated query/reply with timeout. Evolve uses archive branching and strict incumbent selection without source modification. Caller provides independent evaluator. Source Python-specific algorithms beyond extracted profile have not been ported. Native build and model execution unverified.
