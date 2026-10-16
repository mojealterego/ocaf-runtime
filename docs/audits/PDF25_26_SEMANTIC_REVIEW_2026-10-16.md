# Audyt semantyczny PDF25 i PDF26 — 2026-10-16

Źródła: `Architektura OmniContext Agent Framework (OCAF) w ..(5).PDF` (21 stron) oraz `...(6).PDF` (21 stron). Status wszystkich nowych wymagań: PLANNED_OR_UNVERIFIED. Brak testów i dowodów produkcyjnych.

## Pokrycie stron
| Strony | PDF25 | PDF26 |
|---|---|---|
| 1–5 | Zenoh SHM, CRDT, OpenTelemetry, MCP, Tree-sitter: powtórzenia | To samo |
| 6–10 | OAuth/PKCE/ZKP, Firecracker/eBPF, L402, RAG/OWASP, PQC/SNN: powtórzenia | To samo |
| 11–13 | Sześć faz, szkielet Rust, OmniDev/OmniUX: powtórzenia, nie dowody | To samo |
| 14–15 | OmniE2E, OmniForge, OmniWeb, OmniStudio/OmniDoc, OmniGov/Fin/Phys: powtórzenia | To samo |
| 16–19 | **Nowość OmniSec**: role, symulowany pentest i automatyczne reguły | **Nowości OmniCog, OmniEvolve, OmniSense** |
| 20–21 | Bibliografia, bez odrębnych obowiązków wykonawczych | Bibliografia, bez odrębnych obowiązków wykonawczych |

## PDF25: OmniSec / OmniRed / OmniBlue
- PDF25-001: kontrakty ról InzynierZwiadu_MCP, OrkiestratorAtaku, Eksploatator_Strix, LowcaZagrozen_eBPF, AnalitykSOC_ZeroTrust; I/O, RBAC, audyt, limity delegacji. Etapy 0.43, 0.49, 0.63.
- PDF25-002: wyłącznie autoryzowany scope testów, allowlist celów, zgoda właściciela, izolacja Firecracker, brak dostępu do produkcji, kontrola egress, limity zasobów i kill switch. Etapy 0.38–0.39, 0.55, 0.98.
- PDF25-003: weryfikacja PoC i wyników skanera z artefaktami oraz testami negatywnymi; zakaz twierdzeń „100% skuteczności” bez danych. Etapy 0.39, 0.63, 0.98.
- PDF25-004: OmniBlue eBPF/aya i SOC triage; wersjonowane, podpisane reguły, staging/canary, rollback, human approval dla zmian produkcyjnych, odporność na false positives. Etapy 0.39, 0.49, 0.63, 0.98.
- **Dowód negatywny:** funkcje `skanowanie_mcp_narzedzia`, `konstruuj_wektor_ataku`, `monitoruj_srodowisko_testowe` zwracają stałe wyniki; `aplikuj_latke_zenoh` tylko wypisuje log. Brak rzeczywistego pentestu, sondy eBPF i wdrożonej poprawki.

## PDF26: OmniCog
- PDF26-001: cztery rozdzielone warstwy working/episodic/semantic/procedural memory, kontrakty typów i polityki retencji. Etapy 0.43, 0.49, 0.63, 0.67.
- PDF26-002: trwały bitemporalny graph store (valid_time/transaction_time), izolacja tenantów, CRDT merge, odtwarzanie as-of, testy konfliktów i awarii. Etapy 0.38, 0.40–0.41, 0.63, 0.98.
- PDF26-003: provenance, zaufanie źródeł, poisoning resistance, prawo do usuwania, wersjonowanie i weryfikacja proceduralnej pamięci przed użyciem. Etapy 0.43, 0.49, 0.55, 0.98.
- **Dowód negatywny:** `HashMap` i filtr `transaction_time <= timestamp` nie stanowią trwałego bitemporalnego magazynu ani point-in-time recovery.

## PDF26: OmniEvolve
- PDF26-004: izolowana pętla mutacji kodu, wersjonowanie kandydatów, deterministyczny test harness, ograniczenia zasobów, reproducible builds, podpisy i SBOM. Etapy 0.39, 0.43, 0.55, 0.63.
- PDF26-005: niezależny adversarial evaluator, testy regresji/bezpieczeństwa, bramka polityki i zatwierdzenie człowieka przed modyfikacją rdzenia/main; rollback i audyt. Etapy 0.49, 0.55, 0.98.
- **Dowód negatywny:** `adversarial_gating` zwraca zawsze `96.5`; próg 95 nie stanowi walidacji.

## PDF26: OmniSense
- PDF26-006: Graph-of-Thought z ograniczonym budżetem AB-MCTS, deterministyczne warunki stop, limity kosztu i czasu, śledzenie hipotez. Etapy 0.43, 0.49, 0.63.
- PDF26-007: Reflexion/JEPA jako opcjonalna predykcja, kalibracja i porównanie z baseline; żadnych deklaracji skuteczności bez benchmarku; kontrola kosztów L402. Etapy 0.49, 0.67, 0.98.
- **Dowód negatywny:** brak implementacji AB-MCTS, JEPA i pomiarów w PDF26.

## Zasady odbioru
Rzeczywisty kod, test, CI SHA, niezależny audyt i operator acceptance są odrębnymi dowodami. PDF jest źródłem wymagań, nie dowodem ich realizacji. Roje nie są dodatkowymi produktami Agents Builder. Zachować 10 wariantów, 215 etapów głównych i 64 techniczne, chyba że aktualny manifest dowodzi inaczej.
