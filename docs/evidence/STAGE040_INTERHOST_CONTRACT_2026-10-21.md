# Stage 0.40 — independent-host HA contract

Added `src/stage040_interhost_contract.rs`. The acceptance boundary requires
three distinct identities, distinct kernel IDs, streaming replication, witness
fencing, power-loss recovery, revoked-key rejection and a SHA-256 recovery
receipt. Local netns or multiple PostgreSQL processes in one kernel remain
insufficient and are rejected by the contract.

This is an implementation contract with unit tests. Production acceptance still
requires evidence from the actual independent hosts.
