# OCAF Runtime — odbiór przez testy inżynieryjne i symulacyjne

**Decyzja właściciela projektu: ZATWIERDZONA 2026-11-15.** Dotyczy `mojealterego/ocaf-runtime`, gałęzi `main`, historycznych etapów Roadmapy oraz odrębnych 20 pozycji `SOURCE-1.55`–`SOURCE-2.06`. Nie zmienia numeracji ani zapisanych wymagań.

## 1. Wiążąca decyzja
- **Zewnętrzny audyt nie jest obowiązkową bramką odbioru OCAF.** Może być dobrowolny. Prawnie lub kontraktowo wymaganej zewnętrznej atestacji nie wolno fikcyjnie zastępować testem.
- Brak niedostępnej infrastruktury (wiele hostów, KVM, HSM, GPU, realne telefony, płatna chmura) **nie blokuje realizacji**. Zastępuj ją deterministycznym modelem i testami inżynieryjnymi w jasno zdefiniowanym zakresie; nie wpisuj fikcyjnych wyników sprzętowych.
- Właściciel zatwierdził **metodę dowodzenia i odbioru symulacyjnego**, a nie automatycznie każde przyszłe wdrożenie, release ani niewykonany test.
- Nie wymagaj płatnego zewnętrznego recenzenta lub audytora dla wewnętrznej bramki symulacyjnej; nie wymyślaj certyfikatu, podpisu, run ID ani metryki.

## 2. Bramka przyjęcia etapu
Każdy element ma mieć działający kod, test i dowód. Za dowód inżynieryjny uznajemy:
1. Requirement ID, ścieżki implementacji, źródłowy commit SHA, konkretne wejścia/wyjścia, parametry i niezmienniki, ziarno losowości (jeśli dotyczy).
2. Rzeczywiste wykonanie testów: pozytywne/negatywne, błędne dane, limity, izolacja tenantów, idempotencja, replay, timeouts, fault-injection, awarie i recovery stosownie do zagrożenia.
3. Odtwarzalne logi i digesty artefaktów, runner, wynik RED/GREEN dla zmian logiki; wykrywanie pustego outputu, błędnego SHA i kodu zakończenia.
4. Symulacja zgodna z rzeczywistym interfejsem lub jawnie niezależny model referencyjny; model nie staje się dowodem natywnej kompilacji ani realnego działania niedostępnego urządzenia.
5. Zasada fail-closed: `UNKNOWN`/`UNVERIFIED`/`FAILURE` ≠ `PASS`.

## 3. Statusy i granice dowodu
- `IMPLEMENTED`: rzeczywisty kod w repozytorium.
- `TESTED_ENGINEERING_SIMULATION`: uruchomiono odtwarzalne testy modelu konkretnego kontraktu.
- `INTEGRATED_SIMULATION`: połączone komponenty zweryfikowano w symulacji z testami awaryjnymi.
- `OPERATOR_ACCEPTED_SIMULATION`: wyniki konkretnego zakresu zostały przedstawione i przyjęte według tej polityki; pozostają ograniczone do symulacji.
- `DEPLOYED` / `PRODUCTION_APPROVED`: wymagają odrębnego dowodu rzeczywistego uruchomienia, autoryzacji i kryteriów środowiska.
- **Nie ma obowiązkowego kroku `EXTERNALLY_AUDITED`.**

Dotychczasowych `EXISTING_PARTIAL`, `PLANNED`, `NOT_IMPLEMENTED` i `production_approved=false` nie podnosi się bez wykonanych testów. Nie obniża się polityk bezpieczeństwa, grantów, wymaganej autoryzacji kosztów ani kontroli efektów.

## 4. Brak zewnętrznej infrastruktury
Jeśli nie ma KVM/Firecracker, GPU, HSM, PostgreSQL na niezależnych hostach, urządzenia mobilnego lub płatnego CI, wykonaj deterministyczne modele interfejsów, awarii, quorum, protokołów, limitów i recovery. Oznacz jako `TESTED_ENGINEERING_SIMULATION`, nie `VERIFIED_REAL_HARDWARE`, `NATIVE_BINARY`, `LIVE_E2E` czy `DEPLOYED`. Używaj testów natywnych wszędzie, gdzie kompilator i środowisko są dostępne.

## 5. Bramka wykonawcza
`scripts/engineering_simulation_gate.py` uruchamia wyłącznie wpisane na stałe, weryfikowalne zestawy testów Python na czystym checkout `main`. Na początek obejmuje ograniczony zakres `SOURCE-2.05` i `SOURCE-2.06`. Nie certyfikuje wszystkich 20 punktów ani produktu jako całości. Nie uruchamia komend dostarczanych przez niezaufany JSON, prompt czy użytkownika.

Przykład:
    python3 scripts/engineering_simulation_gate.py --repo . --head-sha "$(git rev-parse HEAD)" --requirement SOURCE-2.06

Wynik `SCOPED_ENGINEERING_SIMULATION_PASS` oznacza wyłącznie sprawdzone przypadki. Rozszerzaj allowlistę dopiero po implementacji modułów i testów.
