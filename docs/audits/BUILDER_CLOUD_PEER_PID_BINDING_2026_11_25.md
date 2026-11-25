# Agents Builder Cloud — fail-closed Unix peer PID binding

Date: 2026-11-25 (Europe/Warsaw). Repository: mojealterego/ocaf-runtime.
Parent main SHA: `54210043872acd225f8c7aa0d7f92e4dca686698`.
Scope: AB-CLOUD-CODE, AB-CLOUD-NOCODE; AB-OM-12 scoped infrastructure dependency.
Decision: IMPLEMENTED + LOCAL_UNIT_TESTED. NATIVE_RUST_BRIDGE_INTEGRATION_UNVERIFIED. No release approval.

## Threat and invariant

The pre-existing ingress checked Unix socket file owner and mode, then opened a connection and sent bearer credentials to whatever endpoint answered. This did not prove that the server was the bridge process spawned by its supervisor, allowing a substituted listener with the same service UID to receive request envelopes. The strengthened supervisor passes its real `Popen.pid` into the ingress configuration. Immediately after Unix `connect`, the ingress reads Linux `SO_PEERCRED` and requires same UID and exact expected PID before sending bearer token or body. Unsupported kernel/invalid credentials/unexpected PID are denied with HTTP 503 before forwarding.

Standalone ingress, without `--bridge-pid`, checks kernel UID but cannot bind to the intended child; high-assurance deployment must provide `--bridge-pid` or use supervised mode. Socket inode and filesystem owner checks remain, so neither a private path nor same UID alone claims process identity.

## Actual local verification

- Source environment: Linux x86_64, kernel 6.18.57, CPython 3.13.5, OpenSSL available; no Git remote clone, Cargo or Rust toolchain in this environment.
- TDD RED: new module import missing -> `ModuleNotFoundError: scripts.builder_cloud_peer` (confirmed after correct PYTHONPATH). Not a native Rust E2E RED.
- GREEN: `PYTHONPATH="$PWD:$PWD/scripts/tests" python3 -B -m unittest test_builder_cloud_peer.BridgePeerSecurityTests -v` → **8/8 PASS**.
- OPTIMIZED: identical command with `python3 -O -B` → **8/8 PASS**.
- `python3 -m py_compile scripts/builder_cloud_peer.py scripts/tests/test_builder_cloud_peer.py` → **exit 0**.
- Real Linux kernel tests use socketpair and a separately spawned Unix-domain listener process. Wrong PID, wrong UID (controlled negative fixture), invalid PID, truncated credentials, syscall exception, unsupported platform all fail closed.
- Source/order static check before Git commit: `peer.connect` → `verify_bridge_peer` → `write_frame` and `bridge_pid=bridge.process.pid` present; dedicated workflow triggers on six relevant Python source/test paths.
- A live HTTPS ingress negative test was added to `scripts/tests/test_builder_cloud_ingress.py`; its execution requires the complete checkout. Existing `test_builder_cloud_supervisor.py` TLS/lifecycle tests will exercise the pinned PID through the dedicated Cloud workflow after a CI runner becomes available.
- Full repository Python/Rust builds, existing mTLS regression, native Rust bridge E2E, and GitHub CI green result: **NOT EXECUTED here**. These are mandatory before release gate PASS.

## Test identifiers

`BridgePeerSecurityTests.test_expected_pid_requires_real_positive_integer`
`test_invalid_peer_credentials_fail_closed`
`test_mismatched_process_is_rejected`
`test_peer_credentials_syscall_failure_is_rejected`
`test_real_different_process_cannot_impersonate_expected_bridge`
`test_real_unix_socketpair_pins_expected_current_process`
`test_unsupported_kernel_platform_is_denied_not_skipped`
`test_untrusted_peer_uid_is_rejected`

Full-integration regression pending:
`IngressTest.test_unix_peer_pid_is_verified_before_forwarding`;
`IntegrationContractTests.test_ingress_checks_credentials_after_connect_before_write`,
`test_supervisor_pins_actual_spawned_pid`, and
`test_dedicated_cloud_workflow_triggers_on_all_peer_boundary_changes`.

## Cryptographic evidence

`scripts/builder_cloud_peer.py` SHA-256:
`0b5ee37536a267405fc52461eafce98d8757bffeb32a14884a46ed2965b30868`

`scripts/tests/test_builder_cloud_peer.py` SHA-256:
`fc859ef478bea48c990851516870ca3baa741d057fcb3716bb4cb44b1266da64`

`unit-tests.log` SHA-256:
`d68025afb25740e3019527171c5937e7be51bb4a5c18d166135207809ae044dc`

`unit-tests-optimized.log` SHA-256:
`6c4d34aac0ca98f9b9c0b2cccc123353c0d1d841cee7260390f702213ac6dccd`

Git blob SHA for modified integration files and the final commit SHA are available in the repository history. Logs are copied into this audit folder.

## Release assessment

No change to tenant authorization, OAuth validation, OSTc/PQC, or protected distribution flags. No operator approval, production enablement or release. PID+UID binding does not protect against a privileged attacker with process injection or kernel control. It is process-authentication hardening, not proof of full Agents Builder functionality. Status of all ten products and AB-OM requirements is unchanged; GATE-09 and GATE-10 remain OPEN.
