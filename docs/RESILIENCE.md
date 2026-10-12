# OCAF 0.12 — transactional auditing and PostgreSQL failover fault injection

**Status:** experimental resilience hardening. This release does **not**
certify deployment on a real multi-machine cluster and is not a replacement
for PostgreSQL replication, independent network routing or an external
effect fencing protocol.

## Transactional job audit

The PostgreSQL backend writes metadata-only transition events to
`ocaf_pg_job_audit_v1` in the **same transaction** as the corresponding
job state change. Events include `submitted`, `claimed`, `succeeded`,
`failed` and `uncertain`. Recovery of expired leases writes the
`uncertain` event in the same SQL statement that changes state.
A duplicate submission, stale lease or rejected state transition cannot
create a second successful event.

Journal entries include the sequence number, event kind, actor identity
and database-clock timestamp. They deliberately **do not** include
job arguments, results, HMAC secrets or lease tokens. These events provide
operational provenance, **not tamper-proof logs against privileged database
administrators** or independent proof of external side effects.

`PgJobQueue::audit_owned(owner, job_id, limit)` exposes a bounded,
owner-checked read (limit 1–100). The corresponding signed
`RpcAction::GetJobAudit { id, limit }` permits **only** the
authenticated submitting identity to retrieve its job's events.
Unrelated submitters receive an empty event list and worker-only
identities are denied. The existing legacy SQLite RPC backend currently
rejects the audit request; no fabricated log is returned.

To inspect the original submitter's audit in the provided RPC client CLI,
set `OCAF_ROLE=client`, `OCAF_ACTION=audit`, `OCAF_JOB_ID=<uuid>` and
optionally `OCAF_AUDIT_LIMIT=50` (1–100). Use exactly the same
HMAC-authenticated submitter identity and vetted Zenoh TLS configuration
as the original submission.

## Controlled failure injection

`tests/postgres_chaos.rs` uses a real PostgreSQL server to:
- elect one leader from two independent advisory-lock sessions;
- submit a signed durable job and reject its processing on the standby;
- obtain the leader's PostgreSQL backend PID and invoke
  `pg_terminate_backend` from a separate test connection;
- acquire the released lock with the standby;
- prove the old gateway denies requests;
- reject replay of the original signed submission across the leadership
  change, while retaining the original pending job and its audit event.

`tests/postgres_audit.rs` covers shared history across independent DB
connections, idempotent submission, different-owner exclusion,
metadata-only event contents and expiration recovery that never
silently replays work.

These tests are run on PostgreSQL 16 in CI. They model a **database
session crash**, not host power loss, a partitioned distributed network
or PostgreSQL primary failover.

## Deployment and operational limits

The operator is responsible for independent PostgreSQL replication and
quorum/failover, credential lifecycle, a stable DNS/VIP for the active
Zenoh coordinator, TLS/mTLS issuance and handshake verification,
tenant-separated databases, backups, retention and external
fencing of side-effecting services.

In-flight RPC tasks spawned by the Zenoh queryable may still overlap a
leader transition. PostgreSQL job leases prevent stale state commits
when a lease is no longer valid, but cannot undo a remote tool's external
side effects. There is no exactly-once guarantee. **Jobs in the
`uncertain` state require explicit operator reconciliation.**

Next production acceptance gates: machine-level fault injection and
network-partition scenarios, real mTLS connectivity tests with rejected
untrusted certificates, alerting and health instrumentation, database
outage/failover monitoring, controlled audit retention and independent
security review.
