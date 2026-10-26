# OMAt — osobny serwer narzędzi

OMAt jest oddzielnym procesem serwerowym dla narzędzi. OCAF i OMAs mogą
wysyłać żądania przez Zenoh, ale Zenoh nie nadaje uprawnień. OMAt sprawdza
tenant, agenta, wersję manifestu, capabilities, schemat wejścia, limit czasu,
limit wyjścia, idempotency key i ryzyko efektu ubocznego.

## Kontrakt

Implementacja kontraktu znajduje się w `src/omat_server.rs`.

- `OmatRegistry` przechowuje manifesty narzędzi per `tenant_id`.
- `OmatPolicy` jest allowlistą narzędzi i capabilities.
- `OmatRegistry::admit` daje deterministyczny receipt `ALLOW_BOUNDED`.
- `OmatRegistry::execute` kończy się `ExecutorUnavailable`, dopóki nie ma
  osobno zarejestrowanego, zweryfikowanego executora.
- `health` raportuje serwer, wersję protokołu i licznik narzędzi.

Brak executora jest celową bramką fail-closed. Sam wpis w katalogu nie jest
wykonaniem kodu i nie może uruchomić zewnętrznego procesu.

## Uruchomienie docelowe

1. Zbuduj osobny binarny proces OMAt z przypiętym commitem i toolchainem.
2. Załaduj wyłącznie podpisane manifesty narzędzi z przypisanym tenantem.
3. Skonfiguruj endpoint Zenoh; autoryzacja pozostaje w OMAt.
   Dla `ocaf-omat-zenoh` ustaw `OCAF_OMAT_REPLAY_DB` na absolutną ścieżkę
   w katalogu należącym do użytkownika usługi z uprawnieniami `0700`.
   Baza jest tworzona z `0600`; plik istniejący musi mieć ten sam właścicielski
   zakres dostępu. Symlinki, hardlinki i współdzielony katalog są odrzucane,
   ponieważ SQLite zapisuje też pliki WAL/SHM obok bazy nonce.
4. Podłącz executor WASI/Firecracker po przedstawieniu jego własnego digestu,
   receiptu startu, limitów i testów negatywnych.
5. Włącz narzędzie dopiero po kontroli schematu wejścia/wyjścia i rollbacku.

## Kryteria odmowy

OMAt odrzuca cross-tenant, brak capability, wyłączony tool, przekroczony
timeout/output, niezgodne schema, niepoprawny idempotency key oraz żądanie z
`source_access` innym niż `disabled`, gdy OSTc jest wyłączone.

## Weryfikacja

Testy jednostkowe `omat_server` obejmują rejestrację, listowanie, tenant
boundary, blokadę OSTc, side effects, fail-closed executor i health. Test
produkcyjny wymaga osobnego procesu, Zenoh multi-node, podpisu manifestu,
izolowanego executora oraz pozytywnych i negatywnych dowodów E2E.
