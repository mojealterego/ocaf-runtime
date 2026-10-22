# Plan wdrożenia OCAF / OMAs / OMAt / OSTc

Plan jest wykonywalną kolejnością etapów. Grupa kończąca etap pobiera pierwszy wolny etap, ale przejście wymaga dowodu i nie może omijać zależności.

## Etapy główne

1. **Kontrakt agenta** — manifest, wejścia/wyjścia, limity, identyfikator i lifecycle.
2. **Bramka bezpieczeństwa** — OSTc wyłączone, źródło niedostępne, `ACTIVE` odrzucane przy braku dowodu.
3. **OMAs** — osobny serwer agenta, rejestr, authn/authz, sesje i odtwarzanie błędów.
4. **OMAt** — osobny katalog narzędzi, zakresy uprawnień i izolowany executor.
5. **Zenoh** — transport, korelacja, timeouty i kontrola pochodzenia komunikatu.
6. **Pakowanie** — instalator, checksum, manifest paczki, uninstall i rollback.
7. **Dokumentacja** — architektura, instrukcja operatora, changelog i dowody.
8. **Builder Core** — typed IR, walidacja, Code↔NoCode tylko dla obsługiwanego podzbioru.
9. **Warianty platformowe** — Cloud, Windows, macOS, Android, iOS; każdy ma osobny build i test platformy.
10. **OSTc** — dopiero po dostarczeniu specyfikacji, implementacji i testach kryptograficznych.

## Bramka wyjścia

Etap może mieć status `COMPLETE_VERIFIED` wyłącznie z: artefaktem, digestem, commitem, testem oraz recenzją. Do czasu spełnienia warunków wpis pozostaje `IN_PROGRESS`, `PLANNED` albo `BLOCKED`.
