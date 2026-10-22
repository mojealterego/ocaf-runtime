# Recovery status — 2026-10-21

Ten raport zastępuje wcześniejsze niepotwierdzone deklaracje ukończenia.

## Stan zmierzony

- Manifest: `333` etapów głównych, `64` technicznych, `33` referencyjnych.
- Wczytana tablica szczegółowa: `251` wpisów głównych.
- Aktywne procesy agentów runtime: `0`.
- Aktywne procesy rojów: `0`.
- Make: brak potwierdzonego scenariusza, wykonania i logu.
- Cargo/rustc: niedostępne w bieżącym środowisku.

## Statusy szczegółowej tablicy

- `ACTIVE_TECHNICAL_OPEN`: **3**
- `COMPLETED_BY_OPERATOR_TECHNICAL_ACCEPTANCE_OPEN`: **1**
- `HISTORICAL_NEEDS_VERIFICATION`: **25**
- `PARTIALLY_VERIFIED`: **2**
- `PLANNED`: **151**
- `PLANNED_OR_UNVERIFIED`: **58**
- `UNSPECIFIED_IN_CURRENT_REPO`: **11**

## Zasada odbudowy

1. Najpierw odtwarzamy repozytorium i roadmapę z snapshotu.
2. Następnie budujemy jeden etap w kolejności zależności.
3. Każdy etap wymaga commita, builda, testu, digestu i receiptu.
4. Dopiero po tym przechodzi do kolejnego etapu.
5. Żaden agent ani rój nie otrzymuje statusu pracy bez logu wykonania.

## Dzisiejszy postęp kontraktowy

- Stage 0.39.8: lifecycle microVM claim → jailer → eBPF → VM → receipt → teardown.
- Stage 0.40: niezależne hosty HA mają kontrakt fail-closed w
  `src/stage040_interhost_contract.rs`; realne hosty, witness i power-loss
  recovery nadal wymagają środowiska docelowego.
