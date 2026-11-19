# OCAF Agents Builder — natywna powłoka Windows / macOS / Linux / Kali Linux

Ta aplikacja używa **Tauri v2 + Rust** i ładuje istniejący interfejs `../agents-builder-web`. Jest to wspólny kod źródłowy dla czterech docelowych platform, **nie** zamiennik aplikacji Android/iOS.

## Build

```sh
python3 scripts/generate_builder_icons.py
cargo build --locked --release --manifest-path apps/agents-builder-desktop/src-tauri/Cargo.toml
```

Wynik: `apps/agents-builder-desktop/src-tauri/target/release/ocaf-agents-builder-desktop[.exe]`. Workflow `.github/workflows/agents-builder-native-desktop.yml` buduje osobno na Windows i macOS i publikuje binaria wyłącznie po sukcesie kompilacji. To nie jest podpisany instalator, notarized macOS bundle ani dowód GUI E2E.

## Granice bezpieczeństwa i funkcjonalności

- Tauri capability udziela tylko `core:default`. Brak wtyczek shell, filesystem i HTTP oraz brak zdalnego wykonywania poleceń.
- Edytor Code/NoCode jest ładowany z zasobów lokalnych, bez zewnętrznego CDN.
- **Linux**: natywny transport mTLS do jawnie skonfigurowanego gateway OCAF; nie wymaga CORS ani wystawiania ogólnego klienta HTTP do WebView. Windows/macOS nadal nie mają własnego zweryfikowanego transportu Cloud. Kali wymaga osobnego odbioru.
- Lokalny GGUF runner wymaga oddzielnego procesu i jawnej konfiguracji; nie jest dołączony do powłoki.
- Brak podpisów, automatycznej aktualizacji, instalatora, testów GUI na fizycznych urządzeniach i odbioru produkcyjnego.
- Android i iOS mają odrębne natywne źródła i wymagają osobnego odbioru; Web/Cloud pozostaje usługą wspólną, nie dodatkową aplikacją.

Odbiór zawsze wymaga logów i artefaktów przypiętych do dokładnego SHA. Sam build lub smoke GUI nie zamyka bramki platformowej.

## Linux x86-64: Local/Cloud Code/NoCode

Local: natywny Rust kompiluje ten sam kanoniczny DAG co Runtime,
nie uruchamia narzędzi ani modeli. Eksport zapisuje zwalidowany JSON
w katalogu Pobrane (XDG_DOWNLOAD_DIR), bez nadpisywania. Import Local
tworzy osobny projekt, a import Cloud aktualizuje wyłącznie wybrany
szkic o zgodnej tożsamości i wymaga jawnego zapisu CAS.

Aby połączyć Cloud, operator uruchamia istniejący gateway z
`scripts/builder_cloud_supervisor.py` i konfiguruje przed startem aplikacji:

    OCAF_BUILDER_CLOUD_ORIGIN=https://builder.example:8443
    OCAF_BUILDER_CLOUD_CA=/operator/private/ca.crt
    OCAF_BUILDER_CLOUD_CERT=/operator/private/client.crt
    OCAF_BUILDER_CLOUD_KEY=/operator/private/client.key

Podane wartości to nazwy konfiguracji/ścieżek, nie działające klucze.
Klucz PEM PKCS#8 musi być w prywatnym katalogu właściciela (0700) i
pliku 0600. Klucz jest odczytywany przez powłokę Rust, nie przesyłany
do JavaScript. Użytkownik wprowadza rzeczywisty token OAuth i tenant z
własnej sesji; serwer nadal weryfikuje mTLS, uprawnienia oraz tożsamość.
Brak konfiguracji, niedostępny gateway, obcy tenant lub zły certyfikat
nie otwierają Cloud. Transport ograniczony do GET/PUT projektu i POST
preview, bez proxy, przekierowań oraz automatycznych ponowień.

Uruchomienie testów na izolowanym hoście Linux z Rust, WebKitGTK,
WebKitWebDriver, Xvfb i tauri-driver 2.1.0:

    bash scripts/verify_builder_linux_e2e.sh "$(git rev-parse HEAD)" /absolute/new-evidence "$(command -v tauri-driver)"

Skrypt buduje natywne ELF z Cargo.lock, instaluje kopię do izolowanego
katalogu, uruchamia rzeczywiste GUI Local/Cloud i prawdziwy backend Rust
z tymczasowym PKI/OAuth/SQLite, generuje inwentarz zależności Cargo oraz
ldd i podpisuje dowody jednorazowym kluczem laboratoryjnym; odrzuca
zmodyfikowany podpisany manifest. To nie podpis operatora, publikacja do
produkcji ani potwierdzenie kompatybilności Kali/Windows/macOS.
