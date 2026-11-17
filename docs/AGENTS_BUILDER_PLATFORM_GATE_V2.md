# OCAF Agents Builder — docelowa bramka ośmiu platform

Decyzja operatora 2026-11-17: osiem aplikacji platformowych
(Windows, macOS, Linux, Kali Linux, Android, iOS, HarmonyOS, KaiOS).
Code/NoCode oraz Local/Cloud są trybami; Cloud nie jest osobną aplikacją.
Historyczna macierz dziesięciu produktów pozostaje nienaruszona i nie wystarcza
do odbioru docelowego zakresu. Dotychczasowy skrypt verify_builder_gate10.py
pozostaje dostępny dla audytu historycznych dowodów.

## Wykonanie niezależne od CI

    python3 -m pip install -r scripts/requirements-builder-gate10.txt
    python3 -m unittest discover -s scripts/tests -p test_builder_platform_gate.py -v
    bash scripts/verify_gate10.sh "$(git rev-parse HEAD)" /trusted/evidence <operator-public-key-hex>

Kontrola działa na POSIX z O_NOFOLLOW i O_DIRECTORY, nie wymaga GitHub
Actions. Wrapper verify_gate10.sh wybiera teraz docelową bramkę.

## Kontrakt dowodów v2

W zaufanym katalogu stagingu znajduje się PLATFORM_EVIDENCE.json:
pola dokładnie schema_version (2), git_commit_sha (40 hex), platforms
(tablica ośmiu rekordów) i signature_hex (Ed25519, 128 hex).
Podpis obejmuje kanoniczny JSON (sort_keys=True, ensure_ascii=False,
separators=(',', ':'), allow_nan=False) zakodowany w UTF-8 bez signature_hex.
Klucz publiczny (64 hex) pochodzi od operatora, nigdy z pakietu.

Każdy rekord zawiera platform, distribution, binary, install, sbom, suites.
Każda referencja pliku ma pola path i sha256. Ścieżki są unikatowe,
względne, bez symlinków i traversal. Każdy pakiet ma odrębny skrót,
w tym Linux ELF i Kali DEB. Limit binarium 512 MiB, JSON 8 MiB.
Instalacja: platform, git_commit_sha, binary_sha256, status PASS,
distribution, runner_id, device_id. SBOM: CycloneDX 1.5/1.6 z komponentami.
Cztery suites na platformę mają pola mode, workspace, log.
Log zawiera platform, git_commit_sha, binary_sha256, status PASS,
mode, workspace, runner_id, steps. Kolejne kroki z polami step/status
(PASS): offline dla Local, authenticate i tenant_isolation dla Cloud,
następnie create/edit/validate/compile/export/import.

Weryfikator sprawdza SHA, podpis, macierz przypiętą do bieżącego HEAD main,
integralność i strukturalną kompletność dowodów. Nie uruchamia buildów,
nie sprawdza podpisów instalatorów specyficznych dla systemów, nie wykonuje
GUI, nie poświadcza prawdziwości podpisanych raportów ani tożsamości urządzeń.
Katalog stagingu musi pozostać pod kontrolą zaufanego operatora.
Wynik PLATFORM_EVIDENCE_INTEGRITY_VERIFIED nigdy nie nadaje produkcyjnego PASS.
PLATFORM_CERTIFICATE_RECORD.json jest rejestrem niepotwierdzonych statusów,
nie podpisanym dowodem. Nie należy generować dowodów produkcyjnych z fixture.

## Weryfikacja tego przyrostu

Lokalne Python 3.13 / cryptography 46.0.4: RED brak modułu; GREEN 8/8 testów
na rzeczywistych tymczasowych repozytoriach main, syntetycznych binariach
i podpisanych logach; py_compile PASS. Pliki testu i weryfikatora sprawdzono
przez identyczność Git blob SHA. Testy ani podpisy fixture nie oznaczają
wykonania któregokolwiek z 32 przebiegów na docelowych platformach.
GATE-10 pozostaje NOT_ACCEPTED. Niezależny audyt oraz pełny build niepotwierdzone.

Priorytet wykonawczy: dokończyć wszystkie osiem Agents Builderów zgodnie
z wymaganiami; następnie kontynuować pozostałe otwarte etapy kanonicznej
Roadmapy, bez utraty historycznych wymagań i bez zmian numeracji.
