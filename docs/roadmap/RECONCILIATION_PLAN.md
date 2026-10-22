# Roadmap reconciliation plan

The canonical manifest declares 333 main stages and its detailed `main` array
now contains all 333 records. The previously missing records for stages
`232`–`300` and `321`–`333` were materialized directly from their existing
Roadmap definitions without creating new stage numbers.

The reconciliation preserves the original identifiers. No stage was renumbered
or marked complete: the recovered records remain `PLANNED` or
`PLANNED_OR_UNVERIFIED` according to the source text.

Each recovered record has a source citation, scope, acceptance gate, negative
criterion and explicit status. Code paths, test results, artifact digests and
operator receipts remain open until the corresponding stage is actually built
and verified. The machine-readable plan remains
`reconciliation_plan_v1.json`.
