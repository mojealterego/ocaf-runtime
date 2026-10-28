# Asterion · OCAF-AGENT-001

Asterion is the first named agent package in the 111-agent fleet. It offers
**bounded local stage assistance**: given a JSON array of OCAF stage records,
it uses the shared `ocaf_stage_ledger` crate to propose the next eligible
stages in numeric roadmap order, respecting dependencies and `max_steps`.
An operator can explicitly request one in-memory G01 claim and receive an
updated ledger snapshot on stdout. The input file is never overwritten.

This is a deterministic assistant, not a deployed AI agent. No model, planner
backend, durable memory, Zenoh connection, tool executor, or OSTc service is
provided. The package remains `DRAFT` and `NOT_DEPLOYED`; OMAs and OMAt are
separate disabled endpoints; source access is disabled and `ACTIVE` admission
is denied even if readiness fields are forged.

Run `ocaf-agent-001 status <agent.toml>`,
`ocaf-agent-001 plan <agent.toml> <stages.json>`, or
`ocaf-agent-001 claim <agent.toml> <stages.json> --approve`.
See [USER_MANUAL.md](USER_MANUAL.md) for input format and installation.
