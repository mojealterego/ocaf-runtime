# OCAF 0.9 — local active/passive recovery and strict network preflight

**Scope:** two coordinators on the **same host**, sharing the same private
local SQLite databases and the same stable Zenoh network identity. This
stage is a fail-closed active/passive *process* failover mechanism plus
a transport policy gate. It is **not geographic HA, cross-host election,
replicated storage, consensus, or a certificate authority**.

## Leadership

The coordinator process must receive an absolute
`OCAF_COORDINATOR_LOCK_PATH`, located in the same canonical directory
as `OCAF_JOBS_DB_PATH`. The lock file is opened once and never removed.

- `OCAF_COORDINATOR_MODE=active` (default): attempt to acquire an exclusive
  operating-system file lock. Startup fails if another process owns it.
- `OCAF_COORDINATOR_MODE=standby`: do not open the Zenoh listener or the
  coordinator databases while another coordinator is active. Poll for
  leadership every 250 ms, then open the **same** databases and Zenoh
  endpoint after promotion.
- The operating system releases the lock if the leader crashes or is
  killed. The successor recovers journal state from SQLite WAL.
- The lock is owner-only (0600 on Unix); symlinked lock files and missing
  parent directories are rejected. The parent directory must also be
  private and local.

**Never delete the lock file.** Unlinking and recreating an inode can
produce two independent locks that permit split-brain behavior.

Both processes must use the same absolute paths, node ID, peer keys
and network endpoint configuration. A process supervisor (e.g. systemd)
must restart crashed leaders/standbys; this application does not itself
launch new standby processes. If the host or disk fails, both processes
may fail; local file locking cannot coordinate across machines.
Locking a network filesystem (NFS/SMB) is unsupported.

### Coordinator example

Create a private state directory (e.g. `/var/lib/ocaf`) restricted to
the service account. Provision the peer ACL JSON and the Zenoh config,
then supply identical environment variables to the two local processes:

```bash
OCAF_COORDINATOR_NODE_ID=coord
OCAF_COORDINATOR_LOCK_PATH=/var/lib/ocaf/coord.lock
OCAF_REPLAY_DB_PATH=/var/lib/ocaf/replay.sqlite
OCAF_COORDINATOR_DB_PATH=/var/lib/ocaf/members.sqlite
OCAF_JOBS_DB_PATH=/var/lib/ocaf/jobs.sqlite
OCAF_PEERS_FILE=/var/lib/ocaf/peers.json
OCAF_ZENOH_CONFIG=/etc/ocaf/zenoh-coordinator.json5
```

Launch the first process with `OCAF_COORDINATOR_MODE=active` and a second
with `OCAF_COORDINATOR_MODE=standby`. Each standalone command is
`cargo run --bin ocaf-coordinator` in this prototype. For actual
deployment run built binaries under a supervisor.

## Network policy: secure by default

The `ocaf-coordinator` and `ocaf-rpc-node` binaries validate the
supplied Zenoh JSON5 **before** creating any network session.

Outside a local, explicitly opted-in demo, the validator requires:

- `listen.endpoints` and/or `connect.endpoints` explicitly defined and
  **TLS-only** (`tls/`); all transport protocols explicitly restricted
  to `transport.link.protocols: ["tls"]`.
- `scouting.multicast.enabled=false` and
  `scouting.gossip.enabled=false` to forbid implicit discovery links.
- `transport.link.tls.enable_mtls=true`, a valid absolute CA file path,
  `verify_name_on_connect=true`, and
  `close_link_on_expiration=true`.
- For TLS listeners, absolute paths to a certificate and owner-only key;
  TLS connectors likewise require a client certificate and owner-only key.
- Certificate/key files must exist and be nonempty. Actual chain
  validation and certificate/key matching happen inside Zenoh during
  TLS handshakes; **the preflight does not cryptographically verify
  certificates**.

See `examples/secure-mtls/` for configuration templates (placeholders
must be replaced with real CA/key/cert paths before use).

### Development-only plaintext

Set `OCAF_ALLOW_INSECURE_LOOPBACK=1` **only** when testing on a single
machine with the `examples/insecure-local/` configs. This permits
`tcp/` endpoints *only* with explicit `transport.link.protocols:
["tcp"]`, disabled multicast/gossip, and a literal loopback bind/connect
host (`127.0.0.0/8`, `::1` or `localhost`). Public, LAN, wildcard,
or domain-based TCP endpoints are rejected even with this opt-in.

Do **not** set this variable in deployment service units. HMAC still
does not encrypt anything, and personal data must not cross plaintext
development links.

## Crash acceptance criteria and remaining risks

The repository tests cover mutual exclusion, a standby promotion after
the first guard releases ownership, persistence of queued work across
promotion, rejected symlinks/relative paths, and transport preflight
rejection of malformed, unrestricted and downgraded settings.

This does **not** establish:
- cross-host or cross-region automatic failover;
- a distributed leader consensus protocol or replicated databases;
- no loss of an in-flight RPC response at process death;
- exactly-once external effects or automatic retry of uncertain work;
- workload certificate revocation, live certificate rotation or real
  TLS/mTLS handshakes in a deployed cluster.

Version 0.10 adds a **separate** PostgreSQL advisory leadership API and a
shared PostgreSQL remote-job backend. Neither replaces this CLI's local
SQLite replay and membership journals or activates cross-host coordinator
failover. See [POSTGRES_CLUSTER.md](POSTGRES_CLUSTER.md) for the precise
experimental boundary.

The next architecture milestone is replacing single-host coordination
storage with a replicated transactional backend, introducing a
consensus-safe fencing token at the external resource, and running
multi-machine, certificate-backed chaos/failover tests.
