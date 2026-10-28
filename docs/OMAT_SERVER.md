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

## Trwałe, podpisane odwołanie manifestu

`Stage043ToolStore::revoke_signed` przyjmuje `ToolRevocationRequest` z
`tenant_id`, `tool_id` i dokładnym, małoliterowym `sha256:<64 cyfr hex>`
zarejestrowanego manifestu. Żądanie musi być zapisane w kanonicznym JSON
stanowiącym `payload` istniejącego `SignedOperation` Ed25519 V2. Operacja
wiąże ten payload, tenant, namespace `<tenant>/tools`, target
`<tenant>/tools/control/revoke`, opcode `5`, fuel `1`, nonce, czas wystawienia oraz
`rpc_sender`. Nie ma publicznego API `revoke(tenant, tool_id)` bez podpisu.

Wywołujący musi dostarczyć tenant i klucz publiczny z niezależnej mapy
zaufania oraz tożsamość `authenticated_rpc_sender` z uwierzytelnionej sesji
transportu. Nie wolno brać tych wartości z JSON klienta. Weryfikacja podpisu
oraz 300-sekundowego okna świeżości poprzedza zmianę bazy. SQLite zachowuje
nonce w `stage043_revoke_nonce`; jego zapis i warunkowe ustawienie trwałego
tombstone (`tenant`, `tool_id`, dokładny digest, `revoked = 0`) następują w
jednej transakcji. Błędny podpis, replay, inny tenant/tool, zmieniony payload,
nieaktualny digest, uszkodzony manifest lub błąd bazy odmawiają odwołania.
Poprawnie podpisana próba zużywa nonce również przy odmowie z powodu
nieistniejącego wpisu, starego digestu lub uszkodzenia wpisu; ponowienie
wymaga nowego podpisu i nonce. Błąd transakcji pozostaje odmową fail-closed.
Po restarcie odwołany identyfikator pozostaje niedostępny i nie można go
ponownie zarejestrować pod tą samą tożsamością.

To jest lokalne API trwałego magazynu. Podłączenie go do uwierzytelnionego
transportu i pełne testy E2E nadal są otwarte. Odwołanie nie włącza OSTc,
stanu ACTIVE ani executora.

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
`tests/stage043_persistence.rs` sprawdza także trwałe podpisane odwołanie,
replay po restarcie, wyścig nonce/CAS i negatywne przypadki podpisu, nadawcy,
tenanta, payloadu, czasu oraz nieaktualnego digestu.
