# Stage 0.53 — desktop Linux native build and local GUI E2E / Kali package

Date: 2026-11-19. Source revision: **9ff9a5742a6cf0439d37be7c985a8caed9580b8f** (main); parent d838de37a9bf36780a706a41b3c189246db1ff12. Scope: Linux x86-64 executable and local Code/NoCode functional GUI; unsigned Debian package generated for Kali and its extracted executable tested on Ubuntu. Not a Kali native installation, signed distribution, authenticated Cloud GUI proof, or release approval.

Published source commit: `3d96c8c9af40aea79051aaa10a0cb23d45b0d0a5`; identical complete Git tree `67e473f77ec1756bdcb9f683143bfc824142033c` verified against the original local test commit. Source hashes and receipts refer to the original test checkout.

## Build reproducibility and implementation

The Tauri standalone workspace lacked its own dependency lock. Generated and committed Cargo.lock (428 package entries including workspace package). Windows/macOS and Linux build workflows now require `--locked`; source bundle requires the lock and local native GUI verifier. No change to dependencies in the root runtime lock. No new production capabilities or shell/file/HTTP permissions. Shared editor remains the existing frontend. Cloud remains same-origin HTTPS and not integrated in native Tauri: the local GUI is not allowed to claim Cloud authorization.

Existing isolated Linux host ocaf-stage038-host-b, x86_64, Ubuntu 26.04.1, Rust 1.99.0 (b940084d7), WebKitGTK/WebDriver 2.52.6-0ubuntu0.26.04.2, GTK 3.24.52, Xvfb 2:21.1.22-1ubuntu1.3, tauri-driver 2.1.0. Default archive mirror unavailable; signed Ubuntu indexes and official packages obtained via azure.archive.ubuntu.com, without replacing system source configuration. No GitHub Actions invoked.

## Executed verification

From the committed source revision:

```sh
python3 scripts/generate_builder_icons.py
cargo build --locked --release --manifest-path apps/agents-builder-desktop/src-tauri/Cargo.toml
python3 scripts/verify_native_artifact.py linux apps/agents-builder-desktop/src-tauri/target/release/ocaf-agents-builder-desktop
xvfb-run -a -s '-screen 0 1920x1200x24' python3 scripts/smoke_native_desktop.py apps/agents-builder-desktop/src-tauri/target/release/ocaf-agents-builder-desktop --seconds 5
xvfb-run -a -s '-screen 0 1920x1200x24' python3 scripts/smoke_linux_gui.py apps/agents-builder-desktop/src-tauri/target/release/ocaf-agents-builder-desktop /tmp/linux-gui.png
cargo install tauri-driver --version 2.1.0 --locked
xvfb-run -a -s '-screen 0 1920x1200x24' python3 scripts/verify_builder_linux_local_gui.py --binary apps/agents-builder-desktop/src-tauri/target/release/ocaf-agents-builder-desktop --driver "$(command -v tauri-driver)"
```

First clean dependency compile completed via two bounded invocations (first timed out at 110 s; second resumed, completed in 1m40s). After source commit, locked release build succeeded again. ELF x86-64 verified; ldd has no missing libraries. Process smoke and X11 PNG capture passed. Actual packaged WebKit GUI exercised via W3C WebDriver (not a browser mock, not DOM mutation): create project r1, NoCode transform insertion r2, Code DSL read/modify/apply r3, switch to NoCode and verify budget, dry-run graph, deny unauthenticated Cloud writes, restore Local workspace, restart native process and reopen r3 with nodes/edges/budget unchanged. The inspector/driver is restricted to loopback and run only in this isolated test environment. Temporary XDG directories isolate test data. No remote tools or agents executed.

AB-LINUX-LOCAL-CODE-NOCODE-E2E PASS for native binary. Repeat same test using extracted executable from generated Debian package: PASS on Ubuntu, not Kali. Packaging reproduced byte-identical .deb from same executable and icon, dpkg-deb verifies architecture/content, extracted binary matches input byte-for-byte. No signature, no real Kali host or compatibility claim. Native Cloud certificate/token flow and authenticated mode suites not implemented/verified by this increment.

Regression execution from candidate source (identical Python/GUI sources in commit):
- test_builder_native_desktop: 12/12 (RED: locked-build test failed with unpinned workflows; GREEN after pinning);
- test_builder_bundle_platform_coverage: 3/3 (RED: GUI verifier not required/selected; GREEN);
- test_package_agents_builder: 16/16;
- test_verify_native_artifact: 10/10;
- test_smoke_native_desktop: 3/3;
- test_smoke_linux_gui: 3/3;
- test_smoke_linux_gui_runtime: 7/7;
- test_package_builder_kali: 5/5;
- test_builder_platform_targets: 4/4.
Total 63/63 scoped Python regressions; 2 separately executed native GUI runs. Run each with `python3 -m unittest discover -s scripts/tests -p '<test-module>.py' -v`. No full-runtime CI claim.

## SHA-256

| Artifact | SHA-256 |
|---|---|
| Release Linux ELF | 3b39d839c9ddb9a230e21d2a38a4b023f9e895fccab8c64e681dd7f211b62c05 |
| Unsigned Kali amd64 DEB | ed79367bc70c3f3c458106a001dd17505f92087fbad3c2d1a20689d8f34954f5 |
| Committed Tauri Cargo.lock | f7ae2a6d54dd57180501608030502519f73483f134becf27285ff00e4b9f366f |
| Native GUI verifier source | c2a7921d53711318c47088b59ab220953b92542c3b22497ce44f5c5873d03f84 |
| Linux GUI screenshot | e0fc3977feab515d50b31dc912dc99814554340f0a698d84f79bb500a02b52ed |
| Each of two GUI E2E JSON receipts | f4998378167d81e041822d9e69bdc4f38376290d2625e68808460b4db22ac6dc |

Build artifacts and logs retained in isolated working directory under src-tauri/target/evidence/9ff9a5742a6cf0439d37be7c985a8caed9580b8f; not committed executable blobs. Evidence receipts are unsigned engineering observations, not release certificates. Public release gate remains NOT_ACCEPTED: native Cloud on desktop, actual Kali installation and runtime compatibility, other platforms, signed provenance/SBOM and full 32 suites remain open. Do not count the extracted Ubuntu run as Kali E2E. Existing unrelated global Clippy failures are not resolved here.
