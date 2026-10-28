# User manual · Asterion · OCAF-AGENT-001

## Local build

From this package directory, with Rust installed:

```sh
cargo test --locked
cargo run -- status config/agent.toml
```

The package depends on the sibling runtime's `crates/ocaf-stage-ledger`, so
source builds require the OCAF repository layout. The installable archive
contains a compiled binary and does not require Rust at install time.

## Stage input and commands

Create `stages.json` containing a JSON array of stage records:

```json
[
  {"stage_id":"1","dependencies":[],"status":"READY"},
  {"stage_id":"2","dependencies":["1"],"status":"READY"},
  {"stage_id":"3","dependencies":[],"status":"BLOCKED"}
]
```

Statuses are `READY`, `CLAIMED`, `COMPLETE_VERIFIED`, and `BLOCKED`.
`COMPLETE_VERIFIED` in the input is an operator assertion: this package does
not verify the underlying completion receipt.
`CLAIMED` records also require `claimed_by`. Stage IDs are numeric dotted
components such as `2` or `2.1`. Input is limited to 1 MiB and 4096 records;
the configuration bounds each plan to 1–32 candidates (4 by default).

```sh
cargo run -- plan config/agent.toml stages.json
cargo run -- claim config/agent.toml stages.json --approve
```

`plan` returns eligible candidates and the count of ready stages waiting on
dependencies. `claim` claims the first eligible stage for G01 in memory and
prints `claimed_stage`, `ledger`, and `persisted:false`. It does not alter
`stages.json`. To persist after review, extract the `ledger` array from the
claim output into a new file, for example with `jq '.ledger'`.

## Archive and installation

```sh
packaging/build.sh
OCAF_AGENT_PREFIX="$HOME/.local" packaging/install.sh dist/ocaf-agent-001-asterion-0.1.0.tar.gz
"$HOME/.local/bin/ocaf-agent-001" status "$HOME/.local/share/ocaf-agent-001/agent.toml"
```

`build.sh` runs tests and a locked release build, then creates a tar.gz and a
`.sha256` sidecar. The installer checks the fixed archive layout and internal
SHA-256 checksums before copying the binary and documentation to the prefix.
These checks detect accidental corruption; they are not a publisher signature.
Use `packaging/uninstall.sh` with the same `OCAF_AGENT_PREFIX` to remove it.

Installing the binary does not register the agent with OMAs, allow tools in
OMAt, activate Zenoh, grant source access, or confer certification. Keep the
release configuration fail-closed.
