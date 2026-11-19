# Agents Builder — Git provenance hardening (2026-11-19)

Scope: GATE-10 v2 evidence integrity, not native product acceptance.
Base main commit: `f780c2b587763136843e3e0bc79d3dcd9996e76e`. Base tree: `d743091a0f5359ee9c0b689923e626dfca41b33f`.

Three regression defects reproduced with temporary Git repositories on main:
- Replacement refs could substitute a different matrix under a pinned commit SHA.
- Inherited GIT_DIR/GIT_WORK_TREE could redirect verification to a mirror while the requested checkout had moved to a new commit.
- Git could discover an enclosing repository from an unrelated nested directory containing a copied matrix.

Fix: invoke Git with --no-replace-objects and without inherited GIT_* controls; verify the exact canonical worktree root and full refs/heads/main plus pinned HEAD at both verification boundaries. Existing matrix, Ed25519, path and payload validation stays intact. The test-only git reset/replace runs exclusively in disposable fixture repos.

## Executed evidence

Environment: Linux sandbox, Python 3.13.5, Git 2.47.3, cryptography 46.0.4.
The baseline verifier and test file were copied from the pinned commit and their Git blob IDs compared: a8cd8dd1c9893b1c54c9e64100147774c48b6c1f / 7f60160f2bf75b87c16d4231cf5265697ce043be.

Command (RED and GREEN):
    python3 -m unittest discover -s scripts/tests -p test_builder_platform_gate.py -v

RED: 11 tests, 3 failures, exit 1 (GateError not raised in each new regression).
GREEN: 11 tests, 0 failures, exit 0, 0.815 s.
- test_replacement_commit_cannot_substitute_pinned_matrix: PASS (forged matrix rejected; genuine one still accepted).
- test_git_environment_cannot_redirect_provenance_to_other_repo: PASS.
- test_nested_directory_is_not_the_pinned_repository_root: PASS.
- All eight pre-existing tests: PASS.

    python3 -m py_compile scripts/verify_builder_platform_gate.py scripts/tests/test_builder_platform_gate.py
    git diff --check

Both PASS; the local verification workspace contains only the two source files and fixture repositories, not a full OCAF checkout.

Verified final Git blobs:
- scripts/verify_builder_platform_gate.py: e1708471a08ec1e5aa64953957ce5281a42736e7
- scripts/tests/test_builder_platform_gate.py: 3ac71812705668d5c07e375671b27c260b787117

SHA-256:
- 1f51ef3aca0b8a4e3b351e89899c95e33633526bb5cd701eb9bcde86dd9a97db  scripts/verify_builder_platform_gate.py
- 04f30d09bfcaa2bb232c5c092eeee3f91b23d628c2cbd0dba3a1e98392060fc3  scripts/tests/test_builder_platform_gate.py

## Disposition

SCOPED_PASS: regression and local fixture integrity checks. Full repository tests, Rust build, 32 physical/native platform suites, package signatures and actual release acceptance not executed in this increment. Git and the operator-controlled filesystem remain trusted; no claim of protection from an adversary controlling the verifier or Git executable. Cloud remains a shared mode, not a separate platform. No native target registry or production approval changes. No additional paid infrastructure or CI invoked by this verification.
