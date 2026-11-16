# OCAF Agents Builder iOS — Code + NoCode

SwiftUI native application with WKWebView embedding the existing canonical
offline JSON typed-DAG editor in `apps/agents-builder-web`. XcodeGen adds
that directory as a bundle folder named `builder-web`. A local-file-only
navigation delegate blocks external navigation. Native JSON export opens
the standard iOS share/save sheet; input JSON is size bounded and exported
only following a user action in the shared editor.

Build on macOS with Xcode, XcodeGen and an Apple developer identity:
```sh
cd apps/ios
xcodegen generate
xcodebuild -project OCAFAgentsBuilder.xcodeproj -scheme OCAFAgentsBuilder -sdk iphonesimulator build
```
Then sign and install on a **physical** device and collect genuine GUI E2E.

Limits: Rust UniFFI binding, Keychain, authenticated Cloud ingress, tested
local ES module loading on WKWebView, physical build and full E2E remain
UNVERIFIED. This is a native source application, not GATE-10 certification.
