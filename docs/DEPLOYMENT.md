# OCAF 0.14 — Active/standby deployment and bounded network recovery

**Experimental deployment example.** This release implements a
reconnecting signed Zenoh *submitter/client* and hardened systemd
deployment templates. It does **not** certify production multi-host HA,
mTLS interoperability, automatically fail over a PostgreSQL primary, or
guarantee exactly-once external effects.

## Production-facing client reconnect

`ResilientZenohRpcClient` owns an explicit, operator-reviewed Zenoh
configuration and recreates the Zenoh session on each bounded retry.
The `ocaf-rpc-node` **client** role uses it by default. Each network
attempt opens a fresh Zenoh session and generates a **fresh signed RPC
request ID**. This avoids replay-ledger collisions during recovery.

Defaults: **four** attempts, initial backoff **250 ms** doubling up to
**one second**, and **nine seconds maximum per attempt**. Override only
the number of attempts through `OCAF_RECONNECT_ATTEMPTS=1..8`; invalid
values fail at startup. The bounded policy prevents unending reconnect
loops, and the client can be cancelled by its supervisor.

Automatic retries are limited to:

| Signed RPC action | Retry permitted? | Why |
| --- | --- | --- |
| `Discover`, `Resolve` | Yes | Read-only discovery |
| `GetJob`, `GetJobAudit` | Yes | Read-only, owner-scoped |
| `SubmitJob` | Yes | Persistent owner-bound UUID and identical arguments make duplicate submission idempotent |
| `Execute` | **Never** | External side effects might have occurred |
| `ClaimJob` | **Never** | Claimed leases might already have been issued |
| `FinishJob` | **Never** | A lost result requires reconciliation, not blind reissue |
| `Heartbeat` | **Never** | Mutates membership lease |

A received **verified signed RPC reply**, including an application
failure status, ends the retry cycle. A failed transport exchange or
missing/unverified response may trigger another attempt **only** for
one of the allowed actions above.

### Stable submission UUID is mandatory

The client CLI now **requires `OCAF_JOB_ID`** when
`OCAF_ACTION=submit`. Never generate a new UUID on every retry or
rerun: the original submission might have committed before its reply
was lost. Persist the original UUID and use exactly the same tool,
arguments, authenticated submitter identity and identifier.

```bash
export OCAF_ACTION=submit
export OCAF_JOB_ID="$(uuidgen)"
# Persist the exact UUID outside this process before submission.
cargo run --bin ocaf-rpc-node

# On a lost reply, retry with the *same* OCAF_JOB_ID:
cargo run --bin ocaf-rpc-node

export OCAF_ACTION=status
cargo run --bin ocaf-rpc-node
```

The usual `OCAF_NODE_ID`, `OCAF_PEER_ID`, unique HMAC key and
`OCAF_ZENOH_CONFIG` remain mandatory, and the TLS/mTLS transport policy
is validated **before any connection attempt**. A caller must provide a
stable coordinator address (DNS, virtual IP, or other externally managed
routing); this release does not manage service discovery/load balancing
across machines.

The remote **worker** still uses its existing leased pull loop and
does **not** blindly retry `ClaimJob` or `FinishJob`. Network errors
are surfaced to the worker, and ambiguous work is reconciled through
the existing `uncertain` state. Internal Zenoh reconnection behavior
depends on Zenoh's own transport configuration.

## Late-stage leadership response fencing

The coordinator now validates its dedicated PostgreSQL leadership
session once when admitting a signed request **and again before
emitting a successful signed reply**. This prevents a detected stale
leader from affirmatively acknowledging completed work after losing
its lease/session.

This remains *best-effort response fencing*, **not** external-effect
fencing. An already running tool could still affect external resources.
A lost reply is not proof the operation was cancelled.

## Example systemd service deployment

The `deploy/systemd` directory contains:

- `ocaf-coordinator.service` — a hardened active/passive coordinator
  service with systemd restart-on-failure, resource limits, strict file
  and kernel protections, and no privileged Linux capabilities.
- `ocaf-cluster-health.service` — read-only operator diagnostics run
  as an isolated one-shot task.
- `ocaf-cluster-health.timer` — runs diagnostics periodically and
  exposes warning/critical exit codes to the systemd journal.

**Do not install these units without reviewing and adapting them to
your Linux distribution, paths, credentials and environment.** They
assume binaries installed in `/usr/local/bin`, a dedicated unprivileged
`ocaf` service user, a private `/var/lib/ocaf`, and an explicitly
provisioned `/etc/ocaf/cluster.env`.

A minimal *non-secret* outline of the protected environment file:

```ini
OCAF_CLUSTER_MODE=postgres
OCAF_CLUSTER_ID=my_private_cluster
OCAF_COORDINATOR_NODE_ID=coord
OCAF_COORDINATOR_MODE=standby
OCAF_ZENOH_CONFIG=/etc/ocaf/zenoh-coordinator.json5
OCAF_PEERS_FILE=/etc/ocaf/peers.json
OCAF_CLUSTER_PG_DSN=host=postgres.internal.example port=5432 dbname=ocaf user=ocaf sslmode=require
```

The actual `OCAF_CLUSTER_PG_DSN` must have a securely provisioned
credential; this example intentionally does **not** embed one.
`cluster.env` is sensitive because environment configuration and
process inspection may reveal secrets. Keep it restricted to root and
the service operator, do not print it in logs, and use a trusted secret
manager. `peers.json` and private TLS key files must be accessible to
the `ocaf` account and **owner-only (0600)**; the peer loader enforces
this. Store keys outside home directories because the service enables
`ProtectHome=true`.

Do not expose a plaintext Zenoh or PostgreSQL network in production.
The provided TLS config examples are **templates**: replace them with
real CA-issued node certificates and confirm both successful trusted
handshakes and failed untrusted/expired hostname handshakes before
any production rollout.

Recommended deployment sequence:

1. Provision PostgreSQL HA/backup **outside OCAF**, separate databases
   for different trust domains, and a dedicated least-privileged
   runtime DB role (current automatic table bootstrap may additionally
   require schema-owner privileges; split this in a later release).
2. Create the restricted `ocaf` Unix service account and directories.
   Review systemd hardening compatibility with Zenoh and your host.
3. Install reviewed release binaries and systemd files; provision
   `cluster.env`, HMAC peer policies and real TLS/mTLS credentials.
4. Configure *identical cluster identifiers and peer policies*
   across two hosts but appropriate local listener certificate paths.
   Set both coordinator instances to `standby`: whichever obtains
   PostgreSQL advisory leadership starts serving; the other waits.
5. Configure a **stable client endpoint** externally (DNS, VIP or
   reverse proxy) and validate the client can reconnect after a real
   failover. A PostgreSQL lock alone cannot redirect TCP sessions.
6. Attach systemd/Journald failure notification or an external alert
   manager to the health timer; the timer does **not** send alerts by
   itself. Confirm that no secret data appears in operational output.

Run the health CLI and verify the advisory leader count is exactly one.
Do not expose the health CLI as an unauthenticated HTTP service.

## Verification and known gaps

`tests/transport_reconnect.rs` exercises a bounded retry budget,
exponential backoff, per-attempt timeouts and **no automatic retry**
for effects or lease operations.
`tests/zenoh_reconnect_e2e.rs` checks a real TCP Zenoh server appearing
after the client initiates an authenticated discovery request.
`tests/postgres_coordinator_state.rs` checks that a formerly active
gateway cannot confirm leadership after releasing its PostgreSQL lock.

**Still outstanding:** verified production TLS/mTLS handshakes;
multi-machine network-partition and host-power-off tests; external
client endpoint failover; independently configured PostgreSQL
replication/failover; recovery orchestration for ambiguous worker
completions; signed workload attestation, certificate rotation and
operationally reviewed privileged service isolation.
