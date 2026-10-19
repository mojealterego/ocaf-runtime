# Engineering factory execution slice

`scripts/engineering_factory_runner.py` accepts a workflow supplied mission JSON,
starts at least two separate Codex CLI processes in distinct Git worktrees, checks
their nonoverlapping file scopes, applies their patches to a fresh integration
worktree, runs `cargo test`, then starts a third, read-only Codex reviewer. It
only writes `integrated.patch` and `receipt.json`; a separate publisher must
validate both artifacts before creating a draft PR.

Example mission (the workflow gateway must pin `base_sha` to the checked out
commit):

```json
{
  "mission_id": "ocaf-123",
  "base_sha": "0123456789abcdef0123456789abcdef01234567",
  "workers": [
    {"id": "engineer-a", "role": "rust", "instructions": "Implement module A", "write_set": ["src/module_a.rs"]},
    {"id": "engineer-b", "role": "test", "instructions": "Add tests for module B", "write_set": ["tests/module_b.rs"]}
  ],
  "verify_commands": [["cargo", "test", "--locked", "--no-default-features"]]
}
```

Run with `python3 scripts/engineering_factory_runner.py --mission-file mission.json --output-dir artifacts`.
The process needs `codex`, `git`, `cargo`, and `OPENAI_API_KEY` in its environment.
The key is passed to Codex for API authentication; `shell_environment_policy.inherit=none`
prevents model invoked shell commands from inheriting it. Test subprocesses also
receive an environment with API keys and tokens removed. Run this on an isolated
ephemeral CI runner with a read-only checkout credential. Model processes can
still read repository files within their workspace and can generate unsafe code;
the patch scope, tests, independent review, and separate publish gate are required.

`src/engineering_factory.rs` supplies a durable SQLite mission/work queue for a
single host supervisor. Work items can depend on completed items. Claims use
`BEGIN IMMEDIATE`, a worker identifier, unique token, attempt number, and a
database-time lease; an expired claim becomes `uncertain` and cannot commit.
That module does not launch processes. The CI runner currently operates from
one mission file and is not wired to the SQLite queue. Neither component is a
distributed coordinator or a production authorization boundary.
