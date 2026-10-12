# OCAF 0.24 — network failure and chaos engineering

## Automated scenario suite

The following tests run in the ordinary Rust CI with a genuine
PostgreSQL 16 service and real Zenoh sessions, and are additionally
run as independently named CI steps:

| Scenario | Fault injection | Required outcome |
| --- | --- | --- |
| PG1: hard database session loss | Proxy drops every existing PostgreSQL TCP connection for leader A; accepts no new connections | Standby B wins a strictly newer epoch, A loses ability to write, its worker lease is immediately marked `uncertain`, no automatic retry |
| PG2: healed partition | Re-enable proxy after B is active; open fresh session C | C cannot become leader while B is alive; can acquire only after B releases; generation increases |
| PG3: PostgreSQL startup blackhole | TCP handshake succeeds, proxy silently retains bytes for >5 seconds | `PgLeader::connect` times out fail-closed, no advisory lock/epoch publication occurs; next healthy connection can elect normally |
| Z1: coordinator and transport loss | Abort coordinator A's real Zenoh queryable and terminate its PostgreSQL advisory-lock backend | B becomes leader and reopens a separate queryable on a fresh Zenoh session |
| Z2: client reconnection and replay | Recreate Zenoh session and reissue the same job UUID/payload | The pending job is visible, no duplicate job or duplicate submission audit event is created |

Sources: `tests/postgres_network_chaos.rs`,
`tests/support/pg_fault_proxy.rs`, and
`tests/zenoh_chaos_takeover.rs`.

### Run locally (explicit test requirements)

Provide an **isolated**, disposable PostgreSQL instance on local
loopback. The network-chaos proxy rejects non-loopback or TLS-mode
changes. The example credentials are for a disposable local test
database, never for production.

```sh
export OCAF_TEST_PG_DSN='host=127.0.0.1 port=5432 user=ocaf password=LOCAL_ONLY dbname=ocaf_test sslmode=disable'
export OCAF_REQUIRE_PG_TESTS=1
cargo test --no-default-features --test postgres_network_chaos -- --test-threads=1
cargo test --test zenoh_chaos_takeover -- --test-threads=1
cargo test
```

No chaos test intentionally terminates the entire PostgreSQL primary.
PostgreSQL `pg_terminate_backend` targets only a **dedicated test
leader connection PID**. The proxy binds `127.0.0.1:0` and forwards
exclusively to the specified local PostgreSQL port. It must never be
used to proxy unrelated services or remote operator production
endpoints. Its loss of connections is ephemeral and is reset when the
test task terminates.

The matrix uses unique randomly generated cluster IDs and job IDs to
avoid asserting on other simultaneous CI test workloads. All
assertions cover named records and leadership generations. Timeouts
are bounded. Network recovery is **not** equated with the successful
execution of arbitrary external effects.

## Physical multi-host acceptance matrix — not yet executed

Before production HA can be claimed, operators must run the real
deployment with independent Linux hosts and network failure tools,
record time-stamped evidence and satisfy these criteria:

1. **Three hosts:** A and B coordinators, C worker, physically
   independent from the PostgreSQL cluster; require Zenoh mTLS for
   inter-host communication and verified PostgreSQL TLS.
2. **Leader power loss:** terminate host A's process or power after
   obtaining a running lease; prove B election strictly increases
   epoch and that stale A effects are refused. Capture network packet
   timestamps and DB audit rows, not just CLI return codes.
3. **One-way partition:** drop packets from A toward PostgreSQL while
   allowing B and C; verify A cannot authorize new jobs and that
   third-party effect sinks refuse stale permissions after the
   activation window expires.
4. **Dual-primary PostgreSQL challenge:** separately test database
   failover under loss of quorum using a properly fenced external
   PostgreSQL HA mechanism. Confirm two writable primaries never serve
   simultaneously. OCAF does not supply that fencing.
5. **Zenoh path loss:** sever and restore C-to-coordinator traffic,
   inject 100–1000 ms latency and burst loss, inspect retries: only
   read and idempotent submission RPC may be automatically retried.
6. **Cold recovery and durability:** restore PostgreSQL backup and
   sink journal backups; reconcile `uncertain` jobs manually and verify
   no duplicated committed event, replay marker loss or cross-cluster
   access.
7. **Long-run soak:** run repeated failover cycles under load with
   explicit thresholds for p95/p99 latency, recovery time, zero stale
   commits, and memory/connection growth. Record actual numbers;
   no synthetic performance claims are asserted by these tests.

### Risks and limits

The automated tests share one CI runner and one PostgreSQL primary;
they do **not** simulate host/VM resource separation or real network
packet manipulation between machines. A network-isolated old receiver
may still accept a previously signed permit until its maximum
15-second local activation window expires. HMAC, PostgreSQL epoch
checks and the independent SQLite receiver are cooperating components,
not a single globally atomic commit. There is no guarantee of
exactly-once effects in arbitrary external systems, and OCAF does not
implement PostgreSQL replication, Raft or distributed consensus.

A green CI run proves only the listed test assertions, not successful
physical-host validation.
