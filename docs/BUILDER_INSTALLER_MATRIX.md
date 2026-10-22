# Matrix instalatorów Agents Builder

Maszynowy kontrakt instalacyjny dla wszystkich 10 wariantów znajduje się w
[`builder-installer-contracts-v1.json`](builder-installer-contracts-v1.json).

Ten dokument wyznacza osobny gate dla każdego wariantu. Nie tworzy fikcyjnych
plików instalacyjnych: wpis `BLOCKED_PLATFORM_BUILD` oznacza, że nie ma jeszcze
kompilacji i testu na docelowej platformie.

| ID | Platforma / tryb | Wymagany artefakt | Aktualny stan | Następny dowód |
|---|---|---|---|---|
| AB-CLOUD-CODE | Cloud / Code | obraz/archiwum serwera + manifest | `NOT_IMPLEMENTED` | build CI, test tenantów, digest |
| AB-CLOUD-NOCODE | Cloud / NoCode | obraz web + manifest | `NOT_IMPLEMENTED` | build UI, round-trip IR, digest |
| AB-WINDOWS-CODE | Windows / Code | podpisany MSI/EXE | `BLOCKED_PLATFORM_BUILD` | Windows runner + Authenticode |
| AB-WINDOWS-NOCODE | Windows / NoCode | podpisany MSI/EXE | `BLOCKED_PLATFORM_BUILD` | Windows GUI smoke + Authenticode |
| AB-MACOS-CODE | macOS / Code | podpisany/notarized DMG/PKG | `BLOCKED_PLATFORM_BUILD` | Xcode build + codesign/notarization |
| AB-MACOS-NOCODE | macOS / NoCode | podpisany/notarized DMG/PKG | `BLOCKED_PLATFORM_BUILD` | macOS GUI smoke + codesign |
| AB-ANDROID-CODE | Android / Code | podpisany APK/AAB | `BLOCKED_PLATFORM_BUILD` | Gradle build + emulator/device |
| AB-ANDROID-NOCODE | Android / NoCode | podpisany APK/AAB | `BLOCKED_PLATFORM_BUILD` | instrumentation + process death |
| AB-IOS-CODE | iOS / Code | podpisany IPA/archiwum Xcode | `BLOCKED_PLATFORM_BUILD` | Xcode simulator/device + profile |
| AB-IOS-NOCODE | iOS / NoCode | podpisany IPA/archiwum Xcode | `BLOCKED_PLATFORM_BUILD` | UI smoke + sandbox restore |

Każdy wpis przechodzi kolejno: `BUILD → TEST → SIGN → REGISTER → STAGED`. Nie
można oznaczyć wariantu jako ukończony na podstawie dokumentacji lub wspólnego
artefaktu OCAF-AGENT-001. Pakiet `OCAF-AGENT-001` jest jedynym lokalnym artefaktem
instalacyjnym w tym wydaniu i nie zastępuje tych dziesięciu buildów.
