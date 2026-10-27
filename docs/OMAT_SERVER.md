# OMAt — osobny serwer narzędzi

OMAt jest oddzielnym procesem serwerowym dla narzędzi. OCAF i OMAs mogą
wysyłać żądania przez Zenoh, ale Zenoh nie nadaje uprawnień. OMAt sprawdza
tenant, identyfikatory agenta, capabilities, schemat wejścia, deklarowany
limit czasu, limit wyjścia, format idempotency key i ryzyko efektu ubocznego.
Przypięcie wersji manifestu do wywołania i egzekwowanie czasu przez executor
są nadal wymaganiami dalszej integracji.

## Kontrakt

Implementacja kontraktu znajduje się w `src/omat_server.rs`.

- `OmatRegistry` przechowuje manifesty narzędzi per `tenant_id`.
- `OmatPolicy` jest allowlistą narzędzi i capabilities.
- `discover_for` zwraca tylko identyfikator, wersję, digest i typowane pola
  uprawnień; nie przekazuje swobodnego opisu narzędzia jako instrukcji.
  `schemas_for` wydaje schemat wyłącznie dla aktualnego digestu i polityki
  tenanta. Zmiana wersji (`replace_if_digest`) oraz odwołanie
  (`revoke_if_digest`) wymagają digestu poprzedniego manifestu. Odwołany
  manifest znika z discovery i nie jest dopuszczany do wywołań. To jest
  atomowa kontrola lokalnego rejestru w pamięci, nie trwały protokół
  aktualizacji ani źródło zaufania podpisu.
- `OmatRegistry::admit` odmawia, dopóki nie istnieje zaufane potwierdzenie
  OSTc. Flaga `ostc_enabled` w żądaniu nie jest takim potwierdzeniem.
- `OmatRegistry::execute` odmawia przy tym samym ograniczeniu; przyszły
  brak executora skutkowałby `ExecutorUnavailable` po prawdziwym
  potwierdzeniu OSTc.
- `health` raportuje serwer, wersję protokołu i licznik narzędzi.

Brak executora jest celową bramką fail-closed. Sam wpis w katalogu nie jest
wykonaniem kodu i nie może uruchomić zewnętrznego procesu. Schematy z
niezaufanych manifestów są danymi walidacyjnymi; konsument nie może używać
ich pól opisowych jako instrukcji ani pomijać ponownego sprawdzenia digestu
przy przyszłym wykonaniu.

## Uruchomienie docelowe

1. Zbuduj osobny binarny proces OMAt z przypiętym commitem i toolchainem.
2. Załaduj wyłącznie podpisane manifesty narzędzi z przypisanym tenantem.
3. Skonfiguruj endpoint Zenoh; autoryzacja pozostaje w OMAt.
   Dla `ocaf-omat-zenoh` ustaw `OCAF_OMAT_REPLAY_DB` na absolutną ścieżkę
   w katalogu należącym do użytkownika usługi z uprawnieniami `0700`.
   Baza jest tworzona z `0600`; plik istniejący musi mieć ten sam właścicielski
   zakres dostępu. Symlinki pliku bazy i jego bezpośredniego katalogu,
   hardlinki oraz współdzielony katalog są odrzucane,
   ponieważ SQLite zapisuje też pliki WAL/SHM obok bazy nonce.
   Jeśli przodek ścieżki jest symlinkiem, OMAt otwiera wyłącznie kanoniczną
   ścieżkę do sprawdzonego prywatnego katalogu.
4. Podłącz executor WASI/Firecracker po przedstawieniu jego własnego digestu,
   receiptu startu, limitów i testów negatywnych.
5. Włącz narzędzie dopiero po kontroli schematu wejścia/wyjścia i rollbacku.

## Kryteria odmowy

OMAt odrzuca cross-tenant, brak capability, wyłączony tool, przekroczony
timeout/output, niezgodne schema, niepoprawny idempotency key oraz żądanie z
`source_access` innym niż `disabled`, gdy OSTc jest wyłączone.

## Weryfikacja

Testy jednostkowe `omat_server` obejmują rejestrację, listowanie, tenant
boundary, blokadę OSTc, side effects, fail-closed executor, health, zmianę
schematu między discovery i pobraniem oraz odwołanie. Test produkcyjny wymaga
osobnego procesu, Zenoh multi-node, podpisu manifestu, trwałego rejestru
wersji, izolowanego executora oraz pozytywnych i negatywnych dowodów E2E.
