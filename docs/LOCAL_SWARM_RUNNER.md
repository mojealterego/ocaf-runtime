# Local OCAF swarm runner

`scripts/ocaf_swarm_runner.py` is the execution gate for stage work. Make can
remain a control plane, but a group is not allowed to close a stage from prose.

The runner provides:

- SQLite backed stage state and an atomic claim transaction;
- dependency checks before a claim;
- expiring leases that return work to `READY` for another group;
- immutable evidence records containing a digest of worker output;
- a completion gate requiring a commit SHA, at least one CI run, negative tests,
  and an independent review reference.

Initialize a clean local queue from the canonical manifest and explicitly open
the first stage:

```bash
python3 scripts/ocaf_swarm_runner.py \
  --db .ocaf/swarm.sqlite init \
  --manifest docs/roadmap/canonical_v11_inventory.json \
  --ready-id 0.48
```

Claim work without executing it:

```bash
python3 scripts/ocaf_swarm_runner.py \
  --db .ocaf/swarm.sqlite claim \
  --group G6 \
  --specialization "data connectors and provenance" \
  --base-sha "$(git rev-parse HEAD)"
```

After a verified handoff, pass `--stage-id` when the scheduler has selected a
specific next stage. The runner then refuses to silently substitute an earlier
ready stage.

`run` passes the claim, stage requirements, and lease token to a worker through
environment variables. The worker must print one JSON object to stdout. Only a
zero exit code plus complete evidence can transition the stage to
`COMPLETE_VERIFIED`; all other results remain open and are recorded.

An external worker that was started after `claim` can hand its result back with
the same lease token:

```bash
python3 scripts/ocaf_swarm_runner.py --db .ocaf/swarm.sqlite complete \
  --group G29 --stage-id 0.65 --lease-token "$OCAF_LEASE_TOKEN" \
  --result-file evidence.json --exit-code 0
```

The completion command rejects an expired or already-used lease and applies the
same commit, CI, negative-test and independent-review evidence gate as `run`.

The runner does not claim that a remote GitHub CI run exists. The worker or CI
adapter must provide the real run identifier and commit binding. Rust build and
CI verification still require the repository's configured CI environment.

## 33 specialized groups

`docs/agents/swarm_groups_v1.json` is the executable roster. It contains exactly
`G1` through `G33`, with unique specializations. The database seeds this roster
on every initialization. A group can hold one active lease at a time; after its
lease is completed or expires, it can claim the next eligible stage.

Inspect the live assignment state with:

```bash
python3 scripts/ocaf_swarm_runner.py \
  --db .ocaf/swarm.sqlite groups
```

The command reports all 33 groups as `IDLE`, `WORKING`, or `OPEN`, including
the current stage and lease attempt. Unknown group IDs are rejected by
`claim` and `run`.
