# OCAF Runtime — OBOWIĄZUJĄCA DYREKTYWA WŁAŚCICIELA (2026-11-26)

**Projekt:** mojealterego/ocaf-runtime. **Gałąź:** wyłącznie `main`. Dotyczy wszystkich asystentów, agentów i kolejnych sesji, które odczytują instrukcje tego repozytorium.

## 1. AUDYT ZEWNĘTRZNY NIEWYMAGANY
Audyt zewnętrzny, płatny konsultant, recenzent spoza projektu ani zewnętrzna atestacja **nie są obowiązkowymi bramkami wewnętrznego odbioru OCAF**. Weryfikacja logicznie niezależna (testy/verifier/security review) może i powinna odbywać się wewnątrz systemu. Jeśli rzeczywiste prawo lub konkretny kontrakt wymaga zewnętrznej atestacji dla określonego wdrożenia, odnotuj ten odrębny wymóg; nie używaj go jako pretekstu do wstrzymania implementacji i badań.

## 2. ZEWNĘTRZNA INFRASTRUKTURA NIEDOSTĘPNA — TESTY SYMULACYJNE
Jeżeli do etapu potrzebny jest nieosiągalny rzeczywisty host, KVM/Firecracker, wielohostowe PostgreSQL, HSM, GPU, urządzenie iOS/Android, płatny backend/API lub operator chmurowy, **implementuj rzeczywisty kod** i wykonuj **deterministyczne, uruchamialne testy inżynieryjno-symulacyjne** zgodne z kontraktami. Odtwórz błędy, timeout, quorum/fencing, odcięcie sieci, cross-tenant, budżety, replay, chaos i recovery stosownie do wymagań. Stałe seedy/czas, fixtures, asercje negatywne, SHA źródeł, logi i powtarzalne komendy są obowiązkowe.

Dowód ma obowiązkową kwalifikację: `TESTED_ENGINEERING_SIMULATION`, `INTEGRATED_SIMULATION`, ew. `OPERATOR_ACCEPTED_SIMULATION` po decyzji właściciela. Symulacja nie jest dowodem realnego KVM, natywnego builda, żywych hostów, rzeczywistego settlement ani wdrożenia produkcyjnego. **Nie wolno podnosić statusu do `DEPLOYED` ani `PRODUCTION_APPROVED` wyłącznie na podstawie symulacji.** Sprzętową pozostałość zaznacz `REAL_INFRASTRUCTURE_UNVERIFIED`, realizując dostępne podzadania.

## 3. WŁAŚCICIEL PRZEKAZUJE PEŁNY KOD — WYKNONAJ, NIE OPISUJ
Gdy właściciel przesyła gotowy kod, pliki, ZIP, dokumentację lub dokładne wymagania z poleceniem wdrożenia: **odczytaj wejście, porównaj z istniejącym repo, rzeczywiście wprowadź poprawki i integracje, uruchom właściwe testy, napraw błędy, sprawdź diff, zapisz na `main` oraz przekaż SHA**. Nie odpowiadaj tylko planem, raportem, obietnicą czy pseudokodem. Nie pytaj ponownie o zgodę na zwykłe odwracalne zmiany w zakresie polecenia. Nie usuwaj istniejącego kodu, kryteriów Roadmapy lub testów, by pozornie uzyskać PASS.

Brak narzędzi, uprawnień, czasu sesji albo wymaganej odrębnej autoryzacji nieodwracalnej operacji należy opisać precyzyjnie, **po wykonaniu możliwego zakresu**. Nie deklaruj, że wykonanie będzie trwało w tle, jeśli nie uruchomiono rzeczywistego zadania.

## 4. JEDYNY RZECZYWISTY DOWÓD WYKONANIA
Wymagaj `requirement_id`, kodu/diff, commit SHA, test command, runtime/simulator, wyników RED/GREEN, dowodu negatywnego, środowiska, ograniczeń i weryfikacji. Nie przedstawiaj testu nieuruchomionego jako PASS. Rozróżniaj `SOURCE_IMPLEMENTED`, `TESTED_ENGINEERING_SIMULATION`, `NATIVE_TESTED`, `INTEGRATED`, `OPERATOR_ACCEPTED`, `DEPLOYED` i `PRODUCTION_VERIFIED`.

## 5. REGUŁY NIEZMIENNE
Tylko `main`, commit z kontrolą oczekiwanego SHA, zakaz force push i obchodzenia praw dostępu. Bezpieczeństwo tenantów, kryptografia, sekretów, kontrola płatności, podpisów i uprawnień nadal obowiązują. Dokument nie oznacza ukończenia etapów ani automatycznej aprobaty produkcji.

Instrukcje dla innych środowisk: `AGENTS.md` i `.github/copilot-instructions.md`. Procedura szczegółowa: `docs/ENGINEERING_SIMULATION_ACCEPTANCE.md`.
