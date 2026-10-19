# Transport rationale: no Zenoh for this local checker

This OMAt tool is a deterministic local CLI. It reads an operator-supplied filesystem path, never starts an RPC listener, never routes agent tasks, and has no distributed messaging, scheduling, tenant identity or effects. Therefore the architecture does not require Zenoh in this specific validation tool. All networked OCAF workers and OMAs servers must implement authenticated Zenoh where their architecture requires communication, with real runtime tests and negative evidence.
