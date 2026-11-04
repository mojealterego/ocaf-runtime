# OCAF Agents Builder — natywna powłoka Windows / macOS

Ta aplikacja używa **Tauri v2 + Rust** i ładuje istniejący interfejs `../agents-builder-web`. Jest to wspólny kod źródłowy z dwoma niezależnymi natywnymi buildami, **nie** zamiennik aplikacji Android/iOS.

## Build

```sh
cargo build --release --manifest-path apps/agents-builder-desktop/src-tauri/Cargo.toml
```

Wynik: `apps/agents-builder-desktop/src-tauri/target/release/ocaf-agents-builder-desktop[.exe]`. Workflow `.github/workflows/agents-builder-native-desktop.yml` buduje osobno na Windows i macOS i publikuje binaria wyłącznie po sukcesie kompilacji. To nie jest podpisany instalator, notarized macOS bundle ani dowód GUI E2E.

## Granice bezpieczeństwa i funkcjonalności

- Tauri capability udziela tylko `core:default`. Brak wtyczek shell, filesystem i HTTP oraz brak zdalnego wykonywania poleceń.
- Edytor Code/NoCode jest ładowany z zasobów lokalnych, bez zewnętrznego CDN.
- **Cloud workspace nie jest jeszcze zintegrowany natywnie**: przeglądarkowy klient wymaga same-origin HTTPS, a `tauri://localhost` nie jest uwierzytelnionym gateway. Nie przedstawiać Cloud jako działającego w tych binariach.
- Lokalny GGUF runner wymaga oddzielnego procesu i jawnej konfiguracji; nie jest dołączony do powłoki.
- Brak podpisów, automatycznej aktualizacji, instalatora, testów GUI na fizycznych urządzeniach i odbioru produkcyjnego.
- Android i iOS pozostają osobnymi produktami natywnymi do implementacji; Web/Cloud pozostaje usługą wspólną, nie piątą aplikacją.

Status: `NATIVE_SOURCE_ADDED_BUILD_UNVERIFIED`. Przed zmianą statusu należy odczytać logi i artefakty CI przypięte do dokładnego SHA.
