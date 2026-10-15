# OCAF — dyspozycja operatora 0.38 i przejście do 0.39

**Data:** 2026-10-15. **Gałąź:** `main`.

## Zakres decyzji operatora

Operator upoważnił do **administracyjnego zamknięcia backlogu 0.38**, rozpoczęcia prac 0.39 i zdjęcia blokady administracyjnej wdrożenia z jawnym przyjęciem ryzyka. Zapis źródłowy: [`ops/OCAF_RELEASE_OPERATOR_DECISION_2026-10-15.json`](../ops/OCAF_RELEASE_OPERATOR_DECISION_2026-10-15.json).

Dyspozycja o dalszym rozwoju i odstępstwie nie jest równoznaczna z ukończeniem technicznego odbioru A01–A12, wykonaniem produkcyjnego wdrożenia ani podpisaniem opinii przez niezależnego audytora.

## Faktyczny rejestr decyzji

| Kwestia | Stan na 2026-10-15 |
| --- | --- |
| Zamknięcie backlogu 0.38 decyzją operatora | `CLOSED_BY_OPERATOR` (administracyjnie) |
| Rozwój 0.39 | `AUTHORIZED`, prace trwają |
| Blokada administracyjna próby wdrożenia | `administrative_release_blocked=false` |
| Weryfikacja A01–A12 | `INCOMPLETE` |
| Niezależny audyt bezpieczeństwa | `INCOMPLETE` |
| Zweryfikowana gotowość produkcyjna | `verified_production_readiness=false` |
| Potwierdzone wdrożenie produkcyjne | `production_deployed=false` |
| Wynik bramki kryptograficznej na kompletnym pakiecie odbiorowym | Brak zweryfikowanego, niezależnie podpisanego kompletu dowodów; produkcyjne `NO-GO` |

Próby GitHub-hosted CI (mTLS/WASI, tenant, KVM, Aya i Firecracker) są wartościowymi dowodami laboratoryjnymi, ale runnerów efemerycznych **nie wolno** traktować jako dwóch trwałych hostów produkcyjnych. Nie są też podpisanym odbiorem operatora i niezależnego audytora.

## Zachowanie narzędzi

`scripts/release_readiness.py` domyślnie raportuje braki. Jawna ścieżka odstępstwa:

```bash
python3 scripts/release_readiness.py --repo . \
  --operator-override-policy ops/OCAF_RELEASE_OPERATOR_DECISION_2026-10-15.json \
  --ack-unverified-production-risk
```

Może zakończyć się kodem procesu 0 z `administrative_release_unblocked=true`, ale raport **utrzymuje** `production_approved=false`, `technical_evidence_passed=false` i `deployment_executed=false`. Kod procesu 0 w tej ścieżce oznacza uznaną *decyzję administracyjną*, a nie techniczny sukces zabezpieczeń.

`scripts/stage038_production_gate.py` sprawdza poprawność pakietu i podpisów Ed25519 z odrębnych kluczy; samo przejście weryfikacji może potwierdzić integralność deklaracji, nie fizyczną niezależność hostów ani kompetencje audytora. Ostateczne `production_approved` pozostaje `false` do zewnętrznego odbioru.

## Dalsze dowody

W dniu 2026-10-15 pozyskano [dowody rzeczywistego KVM i Aya](STAGE039_REAL_KVM_AYA_PROOF_2026-10-15.md), a następnie [dowód rzeczywistego bootu jailed Firecracker na KVM](STAGE039_REAL_FIRECRACKER_BOOT_PROOF_2026-10-15.md). Próby wykonano na infrastrukturze CI, nie na docelowym systemie produkcyjnym. Dalszy rozwój 0.39 jest dopuszczony; formalny odbiór bezpieczeństwa 0.38 oraz produkcyjny GO pozostają niepotwierdzone.
