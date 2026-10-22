# Korekta statusu projektu — 2026-10-21

## Korekta

Twierdzenie, że wszystkie etapy roadmapy zostały ukończone, było nieprawdziwe.
Nie ma dowodu E2E, podpisanych artefaktów ani niezależnych buildów dla całej
roadmapy. Status `PLANNED`, `PARTIAL` albo `STRUCTURED_NOT_DEPLOYED` pozostaje
obowiązujący, dopóki nie ma własnego commita, builda, testu, digestu i receiptu.

## Stan potwierdzony pomiarem

- Dzisiaj powstały nowe pliki kontraktów, rejestrów, dokumentacji i struktur
  floty; ich daty można sprawdzić przez `find`/mtime.
- 111 pakietów agentów i 33 pakiety rojów mają status strukturalny, nie
  produkcyjny.
- Aktywne procesy runtime: `0`.
- OSTc: `DISABLED`; dostęp źródłowy: `disabled`; `ACTIVE`: `DENY`.
- Build Rust nie został ponownie wykonany, ponieważ w środowisku nie ma
  `cargo`/`rustc`.
- `OCAF Agents MOJEALTEREGO`, `AGENT-FLEET-111`, `SWARM-FLEET-33`,
  `docs/infrastructure` i `docs/recovery-builds` są dowiązaniami do `/tmp`;
  nie należy liczyć ich jako trwałych plików repozytorium. Próba materializacji
  floty w workspace zakończyła się `ENOSPC`.

## Problem z historią Git

Lokalny `.git/objects/info/alternates` wskazuje na nieistniejące
`/tmp/ocaf-gitobjs` i `/dev/shm/ocaf-gitobjs`. Z tego powodu `git log`,
`git status` i `git diff` kończą się błędem `bad object HEAD`. Nie wolno na tej
podstawie twierdzić, że zmiany zostały zapisane w commicie.

## Reguła dalszej pracy

Każdy etap może otrzymać `COMPLETE_VERIFIED` dopiero po zachowaniu commita,
komendy build/test, wyniku, SHA-256 artefaktu i receiptu. Snapshoty roadmapy w
`docs/recovery-builds/` służą do odtworzenia stanu i nie są dowodem ukończenia.
