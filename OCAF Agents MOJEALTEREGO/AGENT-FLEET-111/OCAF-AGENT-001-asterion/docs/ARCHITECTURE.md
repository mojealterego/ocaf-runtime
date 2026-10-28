# Architecture · Asterion · OCAF-AGENT-001

```text
operator-provided JSON stage records
             |
       bounded input parser (1 MiB maximum)
             |
       ocaf_stage_ledger::StageLedger::new
             |
     plan: next_eligible + scratch claims (max_steps <= 32)
     claim: explicit --approve, group G01, one in-memory claim
             |
       JSON output to stdout (no persistent write)
```

The shared StageLedger checks dependency graph validity and claim ordering.
The operator-supplied ledger is trusted for existing `COMPLETE_VERIFIED`
statuses; Asterion does not authenticate or independently verify their
receipts.
The plan clones the ledger, claims candidates in that scratch copy, and never
changes the caller's ledger. It can identify currently independent ready
stages; it cannot assume blocked dependencies will later complete. The claim
command changes only the in-process ledger and prints a snapshot. A separate
operator must decide whether to persist that output. Receipt completion and
evidence verification are outside this package.

`security.rs` validates every loaded configuration against the immutable
OCAF-AGENT-001 identity and release gates. It rejects unknown TOML fields,
OSTc enablement, source access, any `ACTIVE` setting, live remote services,
model backend claims, shared OMAs/OMAt endpoints, and limits above the bounds.
`lifecycle.rs` cannot move this release to `STAGED` or `ACTIVE`. Readiness
assertions do not bypass the disabled OSTc gate.

`zenoh.rs` defines a transport trait and a disabled adapter. Both OMAs and
OMAt have distinct configuration entries but `enabled=false`; the package
never opens a Zenoh session. OMAs registration, OMAt execution, source access,
certificate issuance, and runtime deployment require separate services and
independent evidence.
