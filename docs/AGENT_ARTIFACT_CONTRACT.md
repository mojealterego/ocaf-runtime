# OCAF — kontrakt artefaktów agentowych (obowiązkowy)

Dotyczy **każdego Agenta, Systemu Agentowego, Roju OCAF, narzędzia OMAt i serwera OMAs**. Rejestr `ocaf-components.tsv` wskazuje odrębny katalog każdego komponentu. Wpis w Make.com, prompt, Skill lub katalog wyłącznie z plikami `.md` **nie oznacza zaimplementowanego Agenta OCAF**.

## Struktura każdego komponentu

```text
swarms/g01/
  component.toml
  Cargo.toml
  src/main.rs  (lub src/lib.rs)
  src/zenoh.rs (gdy zenoh_required = true)
  tests/*.rs
  tests/zenoh_transport.rs (gdy Zenoh wymagany)
  docs/ARCHITECTURE.md
  docs/TECHNICAL.md
  docs/USER_GUIDE.md
  docs/OPERATIONS.md
  docs/CERTIFICATION.md
  evidence/acceptance.json (dopiero po niezależnym odbiorze)
```

Identyczna zasada obowiązuje katalogi `agent_systems/`, `swarms/`, `omat/`, `omas/`. Każde wdrożenie zawiera rzeczywisty wykonywalny kod Rust i testy pozytywne/negatywne. Integracja z Zenoh wymaga weryfikacji rzeczywistego transportu, ACL, tożsamości, awarii, replay i interoperacyjności; sama nazwa zależności `zenoh` w Cargo.toml nie stanowi dowodu komunikacji.

## Bramka strukturalna

Niezależny audyt struktury jest napisany w Rust: `scripts/ocaf_agent_artifact_gate.rs`. Można uruchomić go bez Cargo przez `rustc --edition=2021 scripts/ocaf_agent_artifact_gate.rs -o /tmp/ocaf-artifact-gate && /tmp/ocaf-artifact-gate .`. Sprawdza rejestr, unikalne katalogi, źródła Rust, testy i dokumentację. Opcja `--release` wymaga dodatkowo flagi `CERTIFIED` i dowodu odbioru. **Bramka strukturalna nie kompiluje wszystkich 33 implementacji i nie potwierdza autentyczności dowodów.**

## Bramka wykonawcza i certyfikacyjna

Dla każdego komponentu, każdego etapu i jego podetapów należy uzyskać:
1. Rzeczywiste `cargo build --locked`, `cargo test --locked`, Clippy i rustfmt oraz wyizolowany build wszystkich wariantów/funkcji.
2. Testy integracyjne funkcji oraz scenariuszy negatywnych; przy Zenoh: lokalna i wielohostowa komunikacja na rzeczywistych procesach.
3. Niepodrabialnie powiązane ze SHA commita numery wykonania CI, logi, artefakty, wyniki i kontrolę bezpieczeństwa.
4. Niezależny przegląd kodu oraz formalny odbiór operatora, tam gdzie wymaga go Roadmapa.
5. Kompletną dokumentację techniczną, API, użytkową, operacyjną, kompatybilności i certyfikacyjną.

**Status 33 grup Make G01–G33:** G01 ma lokalny pakiet Rust w stanie rozwoju; pozostałe G02–G33 nadal wymagają odpowiednich katalogów i odbioru. Rejestr jest deklaracją wymaganych artefaktów, nie poświadczeniem ich gotowości. Pełna kontrola musi pozostać niespełniona aż do utworzenia i odbioru wszystkich komponentów. Nie zastępować jej usuwaniem wymaganych wpisów.

Nie zmieniać historycznej numeracji etapów Roadmapy. Etap kończy się dopiero po potwierdzonym wykonaniu wszystkich swoich podetapów.
