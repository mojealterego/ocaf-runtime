# Android/iOS local export lifecycle — 2026-11-19

Base main: 64341eff3de47f60e510f192427f78d7b32e52ad. Canonical native hosts apps/android and apps/ios.
Baseline host blobs: Android c3fc51e13f06eb202c70eec22aea4d46dec23851; iOS 128e3a7ad67ccc8f8d6e1ac6c98310b36874c4fc.

Android: use single-flight synchronized ExportSlot instead of a mutable
pending payload; defensive copy and bounded size; consume the slot once on
SAF result, zero the consumed array, cancel on launch failure/activity
destruction. iOS: use one private temporary directory per export, bounded
file name and content, refuse overlapping shares, clean on completed or
cancelled share, absent presenter and deinit. Ignore stale completion URLs;
cleanup failure retains lease (fail closed). Remove WebKit message handler
during view dismantle. Existing Rust validation remains mandatory.

## Local executable evidence

Linux x86_64, Kotlin 1.9.0 / JRE 21.0.12.1, Swift 6.2.1.
RED: compilation of each new standalone contract test without its new
primitive failed with unresolved ExportSlot or LocalExportLease, respectively.
This is a missing-contract RED, not reproduction on a physical phone.
GREEN: both executables compiled and returned exit 0. Swift test uses real
Foundation filesystem writes and reads, not a simulated filesystem.

    kotlinc apps/android/app/src/main/java/com/ocaf/builder/ExportSlot.kt apps/android/tests/ExportSlotTest.kt -include-runtime -d /tmp/ocaf-export.jar
    java -jar /tmp/ocaf-export.jar
    swiftc -parse-as-library apps/ios/Sources/LocalExportLease.swift apps/ios/tests/ExportLeaseTest.swift -o /tmp/ocaf-export-test
    /tmp/ocaf-export-test

Test coverage: pending overwrite, immutable snapshot, consume once,
cancel/reuse, empty/oversize denial (Android); actual bytes, overlap,
cleanup, stale callbacks, traversal/filename and size denial (iOS).

Verified Git blobs of executable test inputs:
- apps/android/app/src/main/java/com/ocaf/builder/ExportSlot.kt: 8fdf37e75c5772eea4a568d95a17a2401741631c
- apps/android/tests/ExportSlotTest.kt: 97b46d6a8814304ed5555014e65469228c688ad4
- apps/ios/Sources/LocalExportLease.swift: 06fa12abc4d8c3ab111d016a1c0134f7e22b152c
- apps/ios/tests/ExportLeaseTest.swift: 2cc1a25db39351ae69fa13df8d80b4a3dae5b79b

SCOPED_PASS for two platform-independent compiled contract executables.
Host integration source reviewed, not compiled with Android SDK or Xcode.
ActivityResult/Share Sheet and WebView runtime, actual Rust FFI, signed
packages, authenticated Cloud, full repo suite and physical GUI E2E
remain unverified. Process kill may leave iOS temporary files until
system cleanup; no guarantee of secure erasure or remote recipient deletion.
No production promotion or manual paid CI invocation.
