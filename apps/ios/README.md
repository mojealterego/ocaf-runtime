# OCAF Agents Builder iOS — native Code + NoCode + shared Rust UniFFI

SwiftUI and WKWebView embed the existing typed-DAG Code/NoCode editor.
Only local file navigation is allowed; signed Cloud OAuth sessions are not
created by this native host. User-triggered JSON export opens the iOS share
sheet, and local preview passes through the existing Rust `builder_core.rs`
through generated UniFFI Swift bindings.

## Native build on macOS

Install Xcode, XcodeGen, Rust iOS targets and the UniFFI Cargo dependencies.
From the repo root, **do not** generate Xcode project before bindings exist:

```sh
bash apps/ios/build.sh iphonesimulator
bash apps/ios/build.sh iphoneos
```

`build.sh` runs Rust unit tests, generates Swift bindings from the host
.dylib, compiles the static iOS target library, generates an Xcode project
and invokes xcodebuild. On-device installation, developer signing and 10/10
GUI E2E evidence are still required for acceptance.

Status remains **NIEPOTWIERDZONE** unless all actual native build, install,
import/export, Code/NoCode roundtrip and security evidence is collected.
WKWebView ES module/localStorage behavior needs a real target test.

## Canonical mobile workflow (2026-11-17)

CI and self-hosted builds now use apps/ios/build.sh rather than the historical
apps/agents-builder-ios host. Simulator builds disable code signing and use
apps/ios/build as DerivedData, with the Rust target matched to the macOS
host architecture; Swift language mode is 5.0. Device builds do not
disable signing. No compiled iOS artifact or live Cloud session verified.

## Export lifecycle (2026-11-19)

One private temporary directory per in-flight share; stale callbacks cannot remove a later export. Completion/cancellation cleans the file; WebKit handler removed on teardown. Swift Foundation contract executable passed on Linux; UIKit/Share Sheet/Xcode integration and process-abort cleanup remain unverified. See [evidence](../../docs/audits/BUILDER_MOBILE_EXPORT_LIFECYCLE_2026-11-19.md).
