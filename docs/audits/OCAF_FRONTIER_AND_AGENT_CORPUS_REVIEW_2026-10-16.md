# OCAF — przegląd kolejnej paczki architektonicznej (2026-10-16)

## Zakres i metodologia
Przegląd dokumentów: *OCAF Frontier 2027–2033* (10 s.), trzy rewizje raportu *OCAF i JARVIS OMEGA INFINITY* (12, 12 i 14 s.), *Taksonomia współczesnych systemów agentowych* (58 s.), *SKILL Miękkie w Kodzie Rust* (51 s.), *Komunikacja* (118 s.). Część materiałów to rewizje lub rozwinięcia, a nie niezależne wymagania. Nie wolno utożsamiać kodu w PDF z kodem uruchomionym w repozytorium.

## Frontier: wymagania i konieczne bramki akceptacji
| Filar | Wymagany mechanizm | Bramka akceptacji | Ocena kodu przykładowego w PDF |
|---|---|---|---|
| OmniThermo-Governor | budżety energii per agent, RAPL, ograniczanie obciążenia | rzeczywisty odczyt energy_uj, obsługa wrap-around i uprawnień, testy przekroczenia budżetu | tylko symulacja cykli CPU |
| OmniProof-STARK | dowody ograniczeń obliczeń, weryfikacja niezależna | realny prover/verifier STARK z negatywnymi wektorami testowymi | SHA-256 commitment i `generate_proof_stub`, nie ZK-STARK |
| Causal Emergence | bezpieczna agregacja agentów bez utraty semantyki | metryka emergencji, test równoważności zadań i możliwość deagregacji | próg średniej entropii, arbitralna redukcja load |
| OmniMarket-L402+ | podpisane stany kanału, rozliczenie M2M | podpisy obu stron, nonce, replay protection, dispute/settlement, testy utraty łączności | tylko przenoszenie sald w pamięci |
| OmniSubstrate | Cranelift/WASM hot paths, opcjonalnie akceleratory | benchmark A/B, walidacja sandbox i rollback | brak implementacji JIT |
| Lean 4 Proof-Carrying RSI | niezależny dowód i bramka hot-swap | walidacja artefaktu Lean, związanie dowodu z hashem binarnym, atomowy rollback | test pojedynczego wejścia, stały hash |

## Systemy agentowe i soft skills
- Taksonomia jest katalogiem wzorców (AutoGPT, SWE-agent, OpenHands, LangGraph, AutoGen, CrewAI, DSPy, Letta, frameworki bezpieczeństwa), nie licencją do twierdzenia, że wszystkie są zintegrowane.
- Wymagania funkcjonalne do mapowania: router narzędzi, agent engineering, RPA, pamięć, tool calling, telemetry feedback, Blue Team, eval harness.
- Dokument o umiejętnościach Rust zawiera prototypy negocjacji zasobów i epsilon-greedy; produkcyjne wymagania to walidacja ofert (w tym NaN), fair scheduling, budżety, brak starvation, trwałość telemetrii, bounded exploration i testy deterministyczne.
- Wymaganie 110+ umiejętności i modułów należy traktować jako katalog do jawnej inwentaryzacji, nie jako dowód 110 implementacji.

## Komunikacja / Edge
Dokument wymienia IrDA, CIR, Bluetooth/BLE, NFC, UWB, Zigbee, Z-Wave, Li-Fi, OCC, VLC, FSO i inne. Zmapować jako opcjonalne adaptery sprzętowe do Edge/IoT z osobnymi feature flags, modelem uprawnień, emulowanymi testami protokołu i sprzętowymi testami akceptacyjnymi. Nie wprowadzać sterowników urządzeń do ścieżki krytycznej Zenoh bez potrzeby.

## Zasady dowodowe
1. Mapowanie wymagań do istniejącego etapu nie dowodzi implementacji.
2. Liczby wydajnościowe z PDF nie są benchmarkiem OCAF.
3. Zamykanie statusu wymaga: wymaganie → źródło i strona → plik kodu → test → wynik CI → operator.
4. Audyt produkcyjny i niezależny odbiór pozostają OPEN do czasu rzeczywistej weryfikacji.

**Status:** review dokumentacyjny, nie certyfikacja produkcyjna.