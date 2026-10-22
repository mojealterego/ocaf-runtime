# Rejestr projektu OCAF / OMAs / OMAt / OSTc

**Stan na:** 2026-10-21 • **Właściciel techniczny:** repozytorium `mojealterego/ocaf-runtime`

| Projekt | Zakres | Stan implementacji | Granica odpowiedzialności | Dowód wymagany przed produkcją |
|---|---|---|---|---|
| OCAF Runtime | kontrakty, IR, lifecycle i wykonanie | PARTIAL | nie emituje samodzielnie certyfikatów ani instalatorów wszystkich platform | testy E2E + artefakt |
| OMAs | serwer agentów | PARTIAL | admisja, rejestr i sesje agentów; bez katalogu narzędzi | protokół serwera + authz + testy tenantów |
| OMAt | serwer narzędzi | PARTIAL | katalog, zakresy i wykonanie narzędzi | manifest narzędzia + izolowany executor |
| OSTc | szyfrowanie źródeł | DISABLED / FUTURE | nie udostępnia źródeł w bieżącym wydaniu | osobna specyfikacja i testy kryptograficzne |
| Zenoh | transport | PARTIAL | komunikacja i synchronizacja, bez omijania authz | test transportu i odtwarzalności |
| Agents Builder 10 wariantów | Code/NoCode na 5 platformach | NOT_IMPLEMENTED | osobny produkt budujący agenty, nie runtime agent | realny build i test platformy |
| OCAF-AGENT-001 | wzorcowy pakiet zgodności | IN_PROGRESS | demonstracyjny agent z blokadą ACTIVE | testy lifecycle + digest + instalator |

## Zasada dowodowa

Opis architektury nie jest dowodem wdrożenia. Status podnosi się dopiero po zapisaniu artefaktu, SHA-256, identyfikatora commita, logu testów i decyzji operatora. Dokumentacja marketingowa i wycena są materiałami opisowymi i nie nadają statusu produkcyjnego.

## Wycena

Wycena końcowa jest osobnym dokumentem szacunkowym. Nie jest certyfikatem, fakturą, ofertą wiążącą ani dowodem ukończenia. Każdy komponent i każdy wariant Buildera musi mieć osobną pozycję, założenia, walutę, datę i status wyceny.
