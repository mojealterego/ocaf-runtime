# OCAF — kontrola 40 PDF względem Roadmapy (2026-10-16)

## Wynik faktycznie wykonanej kontroli
Zindeksowano i odczytano tekst wszystkich **40 plików PDF / 902 stron** (33 unikalne ekstrakty, 809 stron po deduplikacji całych plików). Dla **każdej z 902 stron** utworzono osobny wiersz: nazwa dokumentu, numer strony, SHA-256 tekstu, słowa-klucze architektoniczne, sugerowane istniejące identyfikatory Roadmapy, fragment źródłowy i status pewności. Pełna lokalna macierz: `OCAF_40_PDF_PAGE_ROADMAP_CROSSWALK.csv`, `.json`, `.md`.

**Wynik mechanicznego mapowania:** 697 stron z co najmniej jednym kandydatem etapu, **205 stron bez automatycznego przypisania**, 51 różnych etapów-kandydatów. Każde dopasowanie oznacza tylko *KEYWORD_CANDIDATE_NOT_SEMANTICALLY_VERIFIED*, nie potwierdzenie kompletnego wymagania. Brak dopasowania nie jest automatycznie luką Roadmapy. Deduplikacja hashów pojedynczych stron wskazuje 264 powtarzające się strony, w tym powtórzenia pomiędzy rewizjami.

## Konkretne potwierdzone powiązania architektury
| Obszar z PDF | Etapy w aktualnym manifeście v11 |
|---|---|
| Zenoh, SHM, edge/pico | 0.46, 1.15, 1.17, 1.55, 1.58 |
| MCP i router narzędzi | 0.43, 1.12 |
| Firecracker, Aya/eBPF | 0.39, 1.42, 1.80 |
| CRDT Automerge/Yrs | 0.46, 1.59 |
| OpenTelemetry | 0.47, 1.20, 1.21 |
| L402 i state channels | 0.49, 0.87, 1.49, 1.54, 1.85, 1.86 |
| ZK-STARK | 0.85 |
| Causal Emergence | 0.86 |
| Cranelift / substrate | 0.81, 0.88 |
| Lean4 i proof-carrying RSI | 0.76, 0.89 |
| RAPL, governor energii | 0.82, 0.90, 1.89 |
| Agent catalog / evaluacje | 0.95, 0.98, 1.97, 1.99, 2.00, 2.02 |
| Agents Builder Code/NoCode | 0.50–0.55 |
| Edge, IrDA/BLE/NFC/UWB/Zigbee/FSO | 1.17, 1.58, 0.93 (wyłącznie kandydaci; brak potwierdzonego pokrycia adapterów sprzętowych) |
| Umiejętności negocjacji, bandit routing | 0.95, 0.98 (wyłącznie kandydaci; brak zweryfikowanego katalogu 110+) |

## Otwarte ryzyka zgodności
1. **205 stron nieprzypisanych:** konieczna ręczna klasyfikacja normatywność/tło/nowe wymaganie. Nie wolno ich pominąć.
2. **Niekompletna atomizacja:** jedna strona może zawierać wiele wymagań; 902 wiersze stron nie są pełną listą wymagań.
3. **Katalog 110+ umiejętności:** wymaga osobnych identyfikatorów, semantycznego porównania i weryfikacji rzeczywistych implementacji.
4. **Łączność radiowa/optyczna:** obecne etapy Edge/IoT nie dowodzą pokrycia każdego protokołu z dokumentu 118-stronicowego. Rozważyć jawny nowy etap po ustaleniu docelowego zakresu i testów sprzętowych.
5. **Kod przykładowy z PDF:** stuby STARK, symulowane RAPL, kanały bez kryptografii i fikcyjne dowody Lean4 nie mogą być liczone jako implementacje.
6. **Testy:** brak wyników cargo, pełnego skanu drzewa i infrastruktury, brak niezależnego odbioru.

## Reguła aktualizacji Roadmapy
Nie dodawać dziesiątek etapów na podstawie samych dopasowań słów. Każda rzeczywista luka musi mieć: identyfikator PDF/strona, cytowany obowiązek, rozróżnienie wymaganie vs przykład/hipoteza, porównanie z pełnym istniejącym etapem, kryterium akceptacji i test. Następnie jednocześnie aktualizować `ROADMAP.md`, lustrzaną `docs/ROADMAP_FULL_ECOSYSTEM_2026.md`, manifest `docs/roadmap/canonical_v11_inventory.json`, README i CI.

**Werdykt:** kontrola pełnostronicowa i mechaniczna macierz wykonane; **semantyczny audyt wszystkich wymagań względem całej Roadmapy nadal OPEN**. Nie deklarować ukończenia ani zgodności produkcyjnej.
