# G01 user guide

Build the package with `cargo build -p ocaf-g01-swarm --locked`. All commands are local. Create an operator key pair with `ocaf-g01-swarm keygen operator.pub operator.key`; keep the private file owner-only. Provide a JSON array of `StageRecord` values, for example:

```json
[
  {"stage_id":"900.1","dependencies":[],"status":"READY","claimed_by":null},
  {"stage_id":"900.2","dependencies":["900.1"],"status":"READY","claimed_by":null}
]
```

These IDs are examples and do not claim roadmap completion. Run `ocaf-g01-swarm init ./g01-state plan.json operator.pub`, then `next` or `status` with the store path. To claim, save the following JSON in `claim.json` and run `ocaf-g01-swarm claim ./g01-state claim.json`:

```json
{"swarm_id":"OCAF-SWARM-01","declared_state":"IN_DEVELOPMENT","readiness":false,"ostc_enabled":false,"source_access":"disabled"}
```

After producing a real artifact and reviewing the source/test evidence, run `ocaf-g01-swarm prepare ./g01-state artifact.bin COMMIT_SHA 'cargo test ...' PASS receipt-001 > payload.json`. Optionally append a JSON limits file as the last argument. The operator then runs `ocaf-g01-swarm operator-sign ./g01-state artifact.bin payload.json operator.key > signed.json`. Finally run `ocaf-g01-swarm handoff ./g01-state artifact.bin signed.json`. The command checks the artifact bytes again; only a successful handoff unblocks a dependent stage. `next` then shows the next eligible stage.
