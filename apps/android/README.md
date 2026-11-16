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
