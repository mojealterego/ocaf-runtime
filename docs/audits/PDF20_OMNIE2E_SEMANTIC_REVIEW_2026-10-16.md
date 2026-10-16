# PDF20 — audyt semantyczny: OCAF Rust, OmniDev, OmniUX, OmniE2E

Źródło: „Architektura OmniContext Agent Framework (OCAF) w Języku Rust: Paradygmaty, Skalowalność i Bezpieczeństwo”, plik 20, **24 strony**, 55 pozycji bibliograficznych. Data: 2026-10-16. Status **PLANNED / UNVERIFIED**, o ile kod i testy repozytorium nie dowiodą inaczej.

| Strony | Wymagania | Istniejące etapy | Warunki odbioru i ograniczenia |
|---|---|---|---|
| 1–4 | Zenoh pub/sub, query/reply, SHM TypedLayout, Automerge/Yrs, DAG/delta/Bloom | 0.46–0.47 | rzeczywisty transport, ACL tenantów, zero-copy i konwergencja w partycjach |
| 4–5 | OTLP, tracing Tokio .instrument() | 0.42, 0.47 | rzeczywisty collector, propagacja spanów przez RPC |
| 5–6 | MCP dynamic loading, Tree-sitter/type-sitter | 0.43–0.44, 0.63, 0.67 | schema validation, odporność na injection, CST nie jest walidatorem JSON Schema |
| 7–8 | OAuth PKCE, izolacja poświadczeń, ZKP | 0.44–0.45, 0.63 | fail-closed authZ, rotacja, weryfikacja circuit i public inputs |
| 8–9 | Firecracker/Jailer, Aya/eBPF | 0.39, 0.55, 0.63 | realny KVM i broker; host eBPF nie obserwuje automatycznie syscalls guest |
| 9–10 | L402/Lightning/Macaroons | 0.49 | ledger, real settlement, limity, idempotencja; kod ma mocked_* i stały preimage |
| 10–11 | RAG/OWASP ASI | 0.64, 0.67, 0.98 | rzeczywisty embedding, ACL/provenance; wektor stały nie jest modelem |
| 11–12 | ML-KEM/ML-DSA i Loihi2 SNN | 0.45, 0.91 | negotiated KEX, realny HIL i fallback |
| 12–13 | sześć faz wdrożenia i progi wydajności | 0.39, 0.43–0.49, 0.63–0.67, 0.91 | odtwarzalne benchmarki; liczby <1 ms, 80%, <1 s niepotwierdzone |
| 13–15 | Cargo.toml, OcafAgent::discover_tool | 0.43, 0.46, 0.49 | szkic, nie kompletna implementacja runtime |
| 15–16 | OmniDev: Architect, Security Critic, Firecracker Executor | 0.43, 0.46–0.47, 0.55, 0.63, 0.67, 0.98 | role, rzeczywista mapa repo, patch, sandbox build/test, audit; kod zwraca AST_MAP, git diff, true |
| 17–18 | OmniUX: Figma MCP, FrontendCSTGenerator, WizualnyKrytykWasm | 0.43–0.44, 0.50–0.55, 0.63, 0.67, 0.98 | design IR, React/Svelte/Yew, DOM pixel diff/a11y, rzeczywiste GUI; kod zwraca FIGMA_AST, <Btn/>, 0.99 |
| **18–21** | **NOWE PDF20: OmniE2E** — InzynierSwiftUI, InzynierKotlinCompose, InzynierQA_SelfHealing; wspólny Rust Core, FFI, równoległe natywne projekty SwiftUI i Jetpack Compose, Zenoh/CRDT, xcodebuild i gradlew, Playwright/Appium, diagnostyka i samonaprawiające się E2E | **0.50–0.55, 0.46–0.47, 0.63, 0.67, 0.98** | prawdziwy build i testy na macOS/Xcode dla iOS i Android SDK/Gradle na Androidzie, artefakty i urządzenia; zgodność API/ABI Rust FFI, zarządzanie pamięcią, podpisywanie, audyt, kontrola zmian selektorów. Self-healing nie może automatycznie osłabiać asercji. Firecracker Linux nie uruchomi natywnego xcodebuild macOS. |
| 21–22 | współpraca OmniDev → OmniUX → OmniE2E | 0.43, 0.46–0.47, 0.50–0.55, 0.98 | typowane kontrakty handoff, wersjonowanie artefaktów, CI, end-to-end na docelowych platformach |
| 22–24 | bibliografia | — | referencje, nie dowód integracji |

## Istotna różnica względem PDF19

PDF20 rozszerza system o **OmniE2E**: trzeciego wyspecjalizowanego roju, nie nowy wariant Agents Builder. Dokument wymienia konkretne role: `InzynierSwiftUI`, `InzynierKotlinCompose`, `InzynierQA_SelfHealing`. Przykład implementacji jest atrapą: zwraca `xcodebuild_swiftui_source`, `gradlew_kotlin_compose_source`, wymusza `wymaga_naprawy: true`, a naprawa selektorów jedynie drukuje komunikat. Nie ma dowodu kompilacji, testów E2E ani publikacji.

### Wymagania przekrojowe

- Każda rola: typowane I/O, scoped permissions, limity kosztów i czasu, deterministyczne stop conditions, ślad audytu i recovery.
- Projektowanie → generowanie → kompilacja → testy → recenzja → podpisanie/publikacja tylko po autoryzacji; brak automatycznego uploadu do App Store.
- 10 wariantów Agents Builder (Cloud, Windows, macOS, Android, iOS × Code/NoCode) pozostaje odrębne.
- Metryki sukcesu i stwierdzenia o przewadze nad rynkiem z PDF są niezweryfikowane.
- Zachować istniejącą numerację; nie deklarować IMPLEMENTED, TESTED ani PRODUCTION_APPROVED bez dowodów.
