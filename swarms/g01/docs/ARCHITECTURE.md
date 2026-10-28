# G01 local engineering swarm architecture

G01 is a local, operator-gated stage coordinator for `OCAF-SWARM-01`. An operator supplies a finite dependency graph of numeric stage IDs at initialization. The `ocaf_stage_ledger` crate selects the first eligible stage and enforces claim ownership, dependency order, exact `PASS` test results, and unique receipt IDs. A single G01 claim may be open at a time.

The store is a JSON journal with a schema version, random store ID, pinned Ed25519 operator public key, plan, and ordered claim/completion events. Every read replays the journal into a fresh stage ledger. A completion event is accepted only when its signature verifies and its content-addressed artifact copy still hashes to the signed SHA-256. Writes use an exclusive local file lock, a new temporary file, file sync, atomic rename, and directory sync.

The G01 registry entry retains the future Zenoh requirement. The current crate exposes a transport boundary that always refuses use. It opens no Zenoh session and makes no distributed delivery claim. The executable performs only explicit local CLI operations; it has no model calls, autonomous task selection, source-access executor, or `ACTIVE` admission.
