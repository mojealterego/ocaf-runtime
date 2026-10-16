# OCAF — paczka 2: macierz rozdziałów PDF ↔ etapy kanoniczne (16.10.2026)

## Inwentaryzacja

**11 plików, 252 strony**, każda strona z tekstem (brak pustych stron). Rewizje dokumentacji częściowo powtarzają wcześniejsze rozdziały; poniższa macierz odnosi się do najpełniejszego pliku `..(10).PDF` (33 strony), a nie do sumy wszystkich wersji. Osobny lokalny rejestr `OCAF_BATCH2_252_PAGE_TRACEABILITY.md` zawiera dla każdej strony hash tekstu, nagłówki, domeny, próbkę i podejrzane wzorce kodu.

## Źródła w tej paczce

1. `Architektura OmniContext Agent Framework (OCAF) w .. (1).PDF`
2. `Architektura OmniContext Agent Framework (OCAF) w ..(1) (1).PDF`
3. `Architektura OmniContext Agent Framework (OCAF) w ..(2).PDF`
4. `Architektura OmniContext Agent Framework (OCAF) w ..(3).PDF`
5. `Architektura OmniContext Agent Framework (OCAF) w ..(4).PDF`
6. `Architektura OmniContext Agent Framework (OCAF) w ..(5).PDF`
7. `Architektura OmniContext Agent Framework (OCAF) w ..(6).PDF`
8. `Architektura OmniContext Agent Framework (OCAF) w ..(7).PDF`
9. `Architektura OmniContext Agent Framework (OCAF) w ..(8).PDF`
10. `Architektura OmniContext Agent Framework (OCAF) w ..(9).PDF`
11. `Architektura OmniContext Agent Framework (OCAF) w ..(10).PDF`

## Macierz rozdziałów i kryteriów

**Uwaga:** identyfikatory wskazują pokrycie *zakresu* Roadmapy; nie są dowodem ukończenia kodu ani audytu implementacji. Strony są indeksami PDF (1-based), a nazwy sekcji zostały skrócone dla czytelności.

| Lp. | Zakres wymagania w PDF | Strony wersji (10) | Etapy Roadmapy | Stan weryfikacji |
|---:|---|---|---|---|
| 1 | Zenoh SHM typed zero-copy | 1–2 | 0.46, 1.15, 1.55 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 2 | CRDT Automerge/Yrs | 2–3 | 0.46, 1.18, 1.57 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 3 | OTLP trace propagation | 3–4 | 0.47, 1.20 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 4 | MCP dynamic routing / Tree-sitter | 3–4 | 0.43, 1.33 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 5 | OAuth, Macaroons, ZKP | 4–5 | 0.44, 0.45, 1.41 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 6 | Firecracker / Aya eBPF | 5 | 0.39, 1.42, 1.80 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 7 | L402 payment protocol | 5–6 | 0.49, 1.49, 1.54, 1.85 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 8 | ML-KEM/PQC / Loihi | 6–7 | 0.45, 0.91, 1.38, 1.41 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 9 | OmniDev | 6–7 | 0.56, 1.92 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 10 | OmniUX | 7–8 | 0.57, 1.92 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 11 | OmniE2E | 8–9 | 0.58, 1.92 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 12 | OmniForge | 9 | 0.59, 1.92 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 13 | OmniWeb | 9–10 | 0.60 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 14 | OmniStudio / OmniDoc | 10–11 | 0.61, 0.62 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 15 | OmniGov / OmniFin / OmniPhys | 11 | 0.64, 0.65, 0.66 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 16 | OmniSec | 11–12 | 0.63, 1.92 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 17 | OmniCog | 12 | 0.67, 1.97 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 18 | OmniEvolve | 12–13 | 0.72, 0.74 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 19 | OmniSense | 13–14 | 0.72 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 20 | Meta decision / OmniArbitrage | 14–15 | 0.69 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 21 | Contracts and assertion / OmniContract | 16 | 0.70 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 22 | Graph error memory / speculative planning | 17–18 | 0.67, 0.71 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 23 | Neurosymbolic formal verification / OmniCausal | 19–20 | 0.73, 0.74, 0.76 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 24 | Recursive self-improvement protocol | 21–22 | 0.72, 0.74 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 25 | OmniSleep / OmniPrune | 23–24 | 0.68 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 26 | Latent communication / OmniAdaptive | 24–25 | 0.75 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 27 | Active inference / topological analysis | 25–27 | 0.77 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 28 | Hyperdimensional sparse computation | 28–29 | 0.78 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |
| 29 | Gödel consistency / OmniFractal / OmniNARS | 30–31 | 0.79, 0.80 | WYMAGANIE W PLANIE; KOD NIEPOTWIERDZONY |

## Kryteria blokujące oznaczenie PASS

- **Komunikacja:** Zenoh SHM i CRDT wymagają pomiaru rzeczywistych procesów, replik, podziału sieci i granic tenantów. Przytoczone liczby wydajności nie są benchmarkami OCAF.
- **MCP:** parser składni nie zastępuje egzekucji uprawnień; potrzebne podpisy manifestów, schematy, przypadki negatywne i ochrona przed metadata poisoning.
- **OAuth/ZKP/PQC:** implementacja musi dowodzić faktycznego uwierzytelnienia, weryfikacji podpisu/obwodu i uzgodnionego algorytmu TLS. Sam przykładowy kod nie stanowi dowodu.
- **Firecracker/eBPF:** należy wykazać uruchomienie microVM, kernel BPF ID i realne odrzucenie niedozwolonych działań. Sam tracing syscall nie jest blokadą ransomware.
- **L402:** użycie `mock_paid_preimage` i `mock_macaroon` to atrapowe poświadczenia, nie rozliczenie.
- **OmniDev:** `weryfikuj_w_sandboxie` zwraca stałą `true`, `analizuj_drzewo_skladni` zwraca `AST_MAP`; konieczne realne uruchomienie testów i przegląd patcha.
- **OmniUX:** `FIGMA_AST` i próg `> 0.98` bez udokumentowanego renderu to demonstracja, nie wynik visual regression.
- **Pozostałe roje:** osobne adaptery, zgody, budżety, logi, testy pozytywne/negatywne i dowody rzeczywistych backendów.
- **Badania matematyczne/kognitywne:** rozróżnić hipotezy i demonstratory od zweryfikowanych twierdzeń, dowodów formalnych i pomiarów sprzętowych.

## Status i ograniczenia

Przypisano 29 grup rozdziałów do istniejących etapów; **nie wykryto uzasadnienia do ponownego dodawania samych nazw**. To nie wyklucza brakujących szczegółowych kryteriów. Zestaw 252 stron został zinwentaryzowany strona po stronie, ale **nie ma potwierdzonej kontroli wszystkich atomowych wymagań względem całego kodu i testów repozytorium**. Bez tych dowodów status produkcyjny pozostaje `production_approved=false`.

**Rozbieżność skorygowana:** poprzedni nagłówek Roadmapy deklarował 195 etapów i 259 pozycji, podczas gdy manifest i wiersze zawierały 207 etapów i 281 pozycji. Zaktualizowano nagłówek i końcowy opis obu kopii Roadmapy do 207+64+10=281.
