# 0.40 — actual PostgreSQL failover and single-witness development evidence

## Observed run

The [JSON receipt](evidence/stage040/2026-10-17/postgres16-witness-lab.json)
was collected on 2026-10-17 from **PostgreSQL 16.15**, running three real
postmasters in one disposable Docker container. All three share one Linux
kernel. This is real PostgreSQL execution, not mocked SQL or independent-host HA.

- `pgbench`: **60,737 transactions**, **8 clients**, **30 seconds**,
  **2027.731415 transactions/second** in this local run.
- Both streaming replicas reached primary WAL position `0/7B320A0` before A
  crashed. SHA-256 matched across the full ordered accounts, branches,
  tellers and transaction-history tables, rather than just balance sums.
- A real negative SQL transaction changed two account balances by opposite
  amounts: the total remained equal, the dataset hash changed, and rollback
  restored the original hash.
- A pre-crash transaction used `synchronous_commit=remote_apply` with
  `ANY 1 (ocaf40_b,ocaf40_c)`; both receivers were observed in quorum state.
  Continuous synchronous operation across later promotions was **not** tested.
- A stopped with PostgreSQL `IMMEDIATE`; B promoted and C reparented. B then
  stopped with `IMMEDIATE`; C promoted. Former A rejoined C as a streaming
  standby after a **fresh base backup**, not `pg_rewind`.
- The SQLite single witness issued epochs **1, 2, 3**, denied overlapping
  grants and rejected old A/B writes through the harness. Reopening its
  connection preserved the active lease. Separate regression tests exercise
  contention between actual spawned witness processes.
- Final event IDs were exactly `1,2,3,4`. Every started postmaster was observed
  stopped before publication; temporary state and the Docker container were
  removed. Source hashes, immutable image ID and runtime flags are in the receipt.

## Implementation

`scripts/stage040_postgres_ha.py` collects actual observations.
`scripts/stage040_docker_lab.py` installs no PostgreSQL on the host: it builds
the image from `ops/stage040/Dockerfile`, runs as `postgres` with a read-only
root, no external network, no capabilities, and memory/process limits, then
collects the full JSON before tmpfs disappears. It force-removes only its
uniquely named container after errors/timeouts. The PostgreSQL base is pinned;
the final image ID records the installed Python/package result.

`scripts/stage040_quorum.py` now serializes leases through SQLite transactions,
persists fencing epochs and clock high-water marks, and rejects nonfinite
values, expired/foreign epochs, clock rollback and boot-domain changes. A
denied expiry observation cannot be rolled back to resurrect the old lease.
**It is one witness, not majority consensus or PostgreSQL server-side fencing.**

`scripts/stage040_interhost_live.py` now waits for DDL WAL replay before
reading the challenge table, binds receiver sender address/port to the primary,
records bounded LSNs and rechecks roles, receiver state and kernel IDs after
the challenge. Its injected-SSH tests do not constitute a remote experiment.

The WAL and long-soak document validators now report **consistency of supplied
evidence only**. They do not authenticate observations. They reject missing
measurements, loose LSN/timeline comparisons, lost transaction acknowledgments,
invented durations and an old primary that accepts writes after return.

## Reproduce

Requires Docker access. Only disposable lab data is mutated:

```sh
python3 scripts/stage040_docker_lab.py \
  --report /absolute/private/directory/new-receipt.json \
  --soak-seconds 30 --exercise-witness
python3 -m unittest discover -s scripts/tests -p 'test_stage040_*.py' -v
```

The existing `stage040-real-postgres-ha.yml` CI runs the witness exercise and
checks the full-table digest, negative corruption test and cleanup receipt.

## Acceptance limits

Stage `0.40` remains `ACTIVE_TECHNICAL_OPEN`, `stage_040_complete=false`,
`production_approved=false`. Missing: three independently authenticated host
kernels exchanging live TCP WAL, majority/external write fencing at the actual
database boundary, old-primary restart during a partition, synchronous ACK
preservation through failover, `pg_rewind`/slot/backpressure acceptance, and
the required long-duration soak. A 30-second local run cannot replace these.

Local stage suite: **82 tests passed**. The full Python suite executed **428
tests** and still has **37 failed assertions and 4 errors** in pre-existing
README/roadmap consistency checks (missing/duplicated stage rows, renderer
interface/count mismatch and differing canonical/mirror files). Those failures
are not waived or hidden by this work.
