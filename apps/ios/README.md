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
