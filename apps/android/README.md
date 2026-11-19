# OCAF Agents Builder Android — native Code/NoCode

Android Jetpack Compose activity with strict WebViewAssetLoader and bundled
`apps/agents-builder-web` Code/NoCode editor (same canonical JSON DSL). No
arbitrary external navigation, filesystem bridge or shell. File import uses
Android SAF GetContent; export uses an origin-scoped WebMessageListener and
SAF CreateDocument. Both remain user-initiated.

Build on Android toolchain (JDK 17, Android SDK 35, Gradle 8.9):
```sh
cd apps/android
gradle :app:assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

NOT YET VERIFIED: Android build/device UI E2E, Rust UniFFI, hardware-backed
secret storage, Cloud gateway/OIDC. No GATE-10 pass is asserted.

## Canonical ARM64 and x86_64 build (2026-11-17)

Use JDK 17, Gradle 8.9, SDK 35 and NDK 27.0.12077973
(https://developer.android.com/build/releases/agp-8-7-0-release-notes).
Install Rust aarch64-linux-android and x86_64-linux-android targets, and
cargo install cargo-ndk --version 4.1.2 --locked. Set ANDROID_NDK_HOME to the
pinned SDK NDK directory. From repository root:

    (cd apps/android && gradle --no-daemon :app:assembleDebug :app:assembleDebugAndroidTest)
    python3 scripts/verify_builder_mobile_bundle.py apps/android/app/build/outputs/apk/debug/app-debug.apk --web apps/agents-builder-web
    (cd apps/android && gradle --no-daemon :app:connectedDebugAndroidTest)

The last command requires an explicitly available Android emulator/device.
The instrumentation calls generated Kotlin UniFFI bindings and checks
deterministic preview and foreign-tenant rejection. APK structural checks
require the correct ELF machine for both ABIs and exact shared web assets.
This is not full GUI E2E, production signing or native Cloud support.
Eight verifier fixture regressions passed locally in Python 3.13.5;
Android Gradle, Rust and instrumentation were not run in this environment.

## Export lifecycle (2026-11-19)

The canonical host uses a single-flight, size-bounded ExportSlot. Pending SAF payload cannot be overwritten by another request and is consumed once. JVM contract executable passed; Android SDK/device integration still unverified. See [evidence](../../docs/audits/BUILDER_MOBILE_EXPORT_LIFECYCLE_2026-11-19.md).
