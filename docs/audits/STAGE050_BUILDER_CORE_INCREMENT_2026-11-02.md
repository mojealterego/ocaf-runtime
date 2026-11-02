# OCAF 0.50 — Agents Builder typed IR: przyrost i audyt dowodów

**Data:** 2026-11-02
**Repozytorium:** mojealterego/ocaf-runtime, main
**Code SHA:** ffc9f899094e344936748024b402549c3af3b55f
**CI:** [run #41507192748](https://github.com/mojealterego/ocaf-runtime/actions/runs/41507192748) — completed/success
**Status:** TESTED_LOCAL_PARTIAL; nie INTEGRATED, DEPLOYED ani PRODUCTION_APPROVED.

## Zweryfikowany zakres

| Obowiązek | Implementacja | Dowód |
|---|---|---|
| Kanoniczny typed IR | src/builder_core.rs: schema v1, Input/Transform/Tool/Output, Text/Json/Bytes, porty, krawędzie, rewizja projektu | tests/builder_core.rs |
| Deterministyczny DAG | Topological ordering, odmowa cykli, duplikatów, nieosiągalnych węzłów, fan-in i niepodłączonych wymaganych portów | Negatywne testy Rust |
| Ograniczony Code↔NoCode IR | Kanoniczny JSON Code DSL, strict import/export, stabilny SHA-256 | Roundtrip i permutacje węzłów |
| Uprawnienia i koszty | Trusted tenant z ingress, ścisłe granty, checked cost sum, resource bounds | Negatywne testy tenant/grant/budget |
| CI kontraktowe | .github/workflows/stage050-builder-core.yml | 13/13 testów, fmt, strict Clippy PASS |

## Braki do pełnego odbioru

1. **0.50:** brak trwałego tenant-scoped project store, historii wersji, migracji schematu, szablonów, debugowania i integracji manifest compiler/executor.
2. **0.51:** brak dwóch rzeczywistych interaktywnych edytorów Code i NoCode. JSON DSL nie jest IDE i nie oznacza odwracalności dowolnego Rust.
3. **0.52–0.54:** brak 10 zbudowanych, uruchomionych i przetestowanych produktów Cloud/Windows/macOS/Android/iOS × Code/NoCode.
4. **0.55:** brak połączonej autoryzacji runtime, podpisanych pakietów, sandbox preview, run receipts i user-approved publish.
5. **P0:** etapy 0.38–0.41 zachowują otwarte kryteria techniczne; symulacja nie dowodzi fizycznego KVM ani HA na niezależnych hostach.
6. **GitHub:** gałąź main zgłasza protected=false, mimo dokumentacyjnych odniesień do protected main. Wymagana osobna konfiguracja repozytorium.

## Granica bezpieczeństwa

Funkcja compile() wytwarza wyłącznie nieaktywny plan i hash. Nie wykonuje tool calls, nie wydaje grantów, nie publikuje, nie podpisuje artefaktów ani nie uwierzytelnia wywołującego. trusted_tenant musi pochodzić z zaufanego ingress, nigdy z payloadu. Limity: 64 węzły, 256 krawędzi, 8 portów/węzeł, 32 granty, 128 KiB JSON.

## Kanoniczny status

- 0.50–0.55: PLANNED_OR_UNVERIFIED — bez zmiany.
- BUILDER-CORE-IR: pełne wymaganie nadal NOT_IMPLEMENTED; przyrost TESTED_LOCAL_PARTIAL.
- Dziesięć produktów: NOT_IMPLEMENTED.
- production_approved=false; deployment_type=not_deployed.
