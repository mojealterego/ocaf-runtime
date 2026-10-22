# Roadmap reconciliation plan

The canonical manifest declares 333 main stages, while its detailed `main`
array contains 251 records. The detailed records end at stage `320`; metadata
names `321`–`333` as source reviewed requirements, but those stage records are
not present in the detailed array.

This is a data reconciliation issue. The missing records must be recovered from
the cited source documents with their original identifiers. No stage may be
renumbered, silently inserted, or marked complete during reconciliation.

Each recovered record requires a source citation, scope, dependencies, code
paths, test command and result, artifact digest, and receipt. The read-only
report is `scripts/roadmap_reconciliation_report.py`; the machine-readable
plan is `reconciliation_plan_v1.json`.
