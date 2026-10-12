# OCAF Runtime 0.7 — authenticated agent membership and capability routing

## Architecture

The coordinator maintains a durable *single-host* service directory on
SQLite. Remote agents send signed `Heartbeat` RPC messages over Zenoh.
Authorized requesters issue signed `Resolve { tool }` requests to select
a currently registered node that advertises that capability.

This is **real networked discovery** with separate Zenoh sessions, not a
replicated scheduler, workload attestation, remote job queue, or guarantee
that a selected node can complete a task.

### Protocol

`RpcAction::Heartbeat { tools: Vec<String> }` updates *only the sender's
identity*, which is derived from the per-peer HMAC key after successful
MAC validation. All declared tools must be in that sender's statically
configured ACL. No caller may register capabilities for another identity.

`RpcAction::Resolve { tool: String }` returns a signed
`NodeRoute { node_id, expires_at_ms }` or JSON null. A requester may
resolve only tools permitted by its own peer ACL. Results never contain
peer secrets, connection credentials, or private transport endpoints.

Each heartbeat replaces the node's capability list transactionally and
extends its lease to **15 seconds** from the coordinator's reception
time. Old nodes stop being eligible at their expiry boundary without
trusting agent-provided wall clocks.

For multiple eligible nodes the coordinator applies a durable,
transactional round-robin cursor. This is routing of *candidates*, not
resource reservation or job assignment. The worker and caller must
already share separate credentials for actual RPC execution. The route
may expire between lookup and use.

Rate limits, message size bounds, short RPC TTLs, HMAC verification,
peer allowlists and persistent replay checking from the previous phases
also apply to heartbeat and lookup actions.

### Local topology

```text
            signed Heartbeat (every 5s)
     [agent-a] ---------------------------+
                                          |
     [agent-b] ---------------------------v
                                  [Zenoh coordinator]
                                      | SQLite WAL
                                      | node leases
                                      | per-tool membership
                                      | round-robin cursor
     [client-c] --- signed Resolve ------>|
                 <--- signed NodeRoute ---+
                      |
                      +------ separately authenticated direct RPC ------> agent-a/b
```

The RPC demo server can optionally advertise its `demo_echo` capability
while its local Zenoh server is active. It uses
`OCAF_COORDINATOR_NODE_ID` and `OCAF_COORDINATOR_KEY_B64` to establish
a separate per-coordinator credential.

## Standalone coordinator CLI

Required environment variables for `ocaf-coordinator`:

- `OCAF_COORDINATOR_NODE_ID`: node ID (strict identifier rules).
- `OCAF_PEERS_FILE`: absolute path to trusted, owner-only JSON mapping of
  node names to unique pairwise HMAC keys and capability allowlists.
- `OCAF_ZENOH_CONFIG`: reviewed Zenoh JSON5 network configuration.
- `OCAF_REPLAY_DB_PATH`: absolute SQLite replay-ledger file path.
- `OCAF_COORDINATOR_DB_PATH`: absolute SQLite membership file path.

Use separate SQLite filenames and a private parent directory. The CLI
does not silently revert to an in-memory replay journal.

Peer file format (illustrative; replace placeholders with independent,
securely generated 32-byte keys):

```json
{
  "agent-a": {
    "key_b64": "<BASE64_32_BYTE_KEY_A>",
    "allowed_tools": ["demo_echo"]
  },
  "client-c": {
    "key_b64": "<BASE64_32_BYTE_KEY_C>",
    "allowed_tools": ["demo_echo"]
  }
}
```

Each key must be different for independent trust domains. On Unix
`OCAF_PEERS_FILE` must be owner-only (mode 0600). Never commit a real
configuration containing HMAC keys to GitHub.

Run: `cargo run --bin ocaf-coordinator`.

### Example agent registration

A remote `ocaf-rpc-node` configured as a server uses a separate HMAC
key for its coordinator. Its optional coordinator configuration is:

```bash
OCAF_COORDINATOR_NODE_ID=coordinator
OCAF_COORDINATOR_KEY_B64=<KEY_ASSIGNED_TO_THIS_AGENT>
```

These are **additional** to the node's normal RPC server configuration.
When enabled, the agent emits a heartbeat every five seconds. Failed
heartbeats are logged and retried on the following interval; after
approximately 15 seconds without successful registration, the node
stops appearing in new route lookups. The coordinator uses the sender
identity from the signed request; unrecognized peers are rejected.

For manual test operations, `ocaf-rpc-node` in client mode supports
`OCAF_ACTION=heartbeat` or `OCAF_ACTION=resolve` with `OCAF_TOOL`
(default `demo_echo`). Set `OCAF_PEER_ID` to the coordinator ID and
`OCAF_SHARED_KEY_B64` to the key configured for that requesting node.

## Security and production gaps

**Important:** HMAC authenticates payload integrity; it does **not**
encrypt TCP traffic. The loopback demo Zenoh configs deliberately use
plaintext TCP. For traffic between machines, use properly configured
and independently validated Zenoh TLS/mTLS or an encrypted private
overlay. This code does not enforce transport certificates or rotate
peer credentials.

- An authenticated agent can advertise only its allowlisted names, but
  currently there is no remote workload attestation confirming that its
  executor actually implements them.
- Health is a time-limited *heartbeat hint*, not a measured end-to-end
  execution probe. Slow/unreachable nodes can still be returned.
- This coordinator persists state on **one host only**; it is not
  replicated consensus and is a single point of failure.
- Route responses do not reserve or transfer job leases, and there is
  no transactional cross-host exactly-once dispatch.
- The SQLite directory must be on a reliable local filesystem, secured
  along with WAL/SHM files; do not place it on generic network shares.
- Operator must implement certificate rotation, secure key provisioning,
  deployment attestations, quotas, monitoring and disaster recovery.

## Acceptance tests

`tests/coordinator.rs` covers peer ACL enforcement, bad messages,
restart persistence, lease expiry, capability replacement, replay
suppression and round-robin selection.

`tests/coordinator_zenoh_e2e.rs` establishes three separate Zenoh TCP
sessions for an authenticated coordinator, registering worker and
lookup client. It tests the actual signed remote heartbeat and route
response. This is **loopback integration testing**, not production
multi-host or TLS verification.
