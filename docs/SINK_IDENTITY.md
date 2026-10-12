# OCAF 0.22 — scoped identities and lease-bound sink permits

## Protocol v2 / mandatory namespace

An independent sink is now pinned to `OCAF_SINK_CLUSTER_ID`, its trusted
PostgreSQL leadership namespace. The cluster ID must be a valid OCAF
node-like identifier and matches both signed `ActivateEpoch` messages
and signed `SinkPermit` capabilities. **An epoch number is not globally
unique**; never share secrets across clusters or sinks.

`SinkPermit::from_lease` binds a permit to:
- sink ID + cluster ID + worker ID + job UUID;
- validated tool name, positive attempt and strictly positive epoch;
- SHA-256 digest of the unpredictable lease token (the token itself is
  **not** copied into the permit or sink journal);
- exact JSON payload digest and permit expiration no later than the job
  lease expiration.

The trusted controller's `ZenohSinkClient::issue_permit_for_lease`
additionally obtains the current epoch through a verified PgLeader,
checks the PgJobQueue belongs to the *same* cluster, queries
`validate_current_lease` against current PostgreSQL job and epoch rows,
and bounds the issued permit to at most 15 seconds.

This closes the former helper's ability to issue signed effect permits
from arbitrary caller-supplied job IDs. Low-level `from_lease` only
constructs data; callers with direct access to controller secret must
still be considered highly privileged.

## Migration / operational key custody

Upgrade receiver, issuer and workers together. **v1 sink requests and
permits will be rejected** (protocol v2). Configure and pin the
cluster's authoritative ID on every sink host:

```sh
OCAF_SINK_CLUSTER_ID=<existing OCAF_CLUSTER_ID>
```

Never copy the controller key to worker nodes. Put controller/worker
secrets in mode-0600 or orchestrator-managed secret stores, separate
the service identities, and perform rotation with a coordinated
maintenance window. **Automatic key rollover and zero-downtime
certificate/key rotation are not implemented in this release.**
All network endpoints continue to require vetted TLS/mTLS, except
explicitly enabled loopback-only development mode.

## Security boundary and open risks

The sink can authenticate **who signed** an issuance request and bind
it to the current job identity at *issuance time*. It cannot
independently query the authoritative PostgreSQL lease without
connecting to it, and it cannot atomically validate PostgreSQL and
SQLite together. An old signed permit may remain usable at an isolated
sink until a higher epoch activation arrives or the local activation
window expires. The maximum window remains 15 seconds.

Receiver journal idempotency covers this one JSON event store, not
arbitrary third-party effects. Physical multi-host partition and
certificate lifecycle tests remain required before production HA.

## Optional owner-only key files

The sink executable accepts either the original base64-valued key
environment variable or an explicit private-key **file** variable,
never both simultaneously:

```sh
OCAF_SINK_CONTROLLER_KEY_FILE=/etc/ocaf-sink/controller.key
OCAF_SINK_WORKER_KEY_FILE=/etc/ocaf-sink/worker.key
```

Each file contains a single base64-encoded 32-byte key, optionally
terminated by a newline. On Unix, opening uses `O_NOFOLLOW` on the
final component, requires a regular file of at most 256 bytes,
owner-only permissions, and the effective service-user UID. Relative
paths, symlinks and group/world-readable files are rejected. The keys
must still be unique for each identity and separate from other clusters.
Selecting a key file avoids exposing the raw secret in process
environment variables, but does not offer automatic zero-downtime
rotation or protect against a compromised host.
