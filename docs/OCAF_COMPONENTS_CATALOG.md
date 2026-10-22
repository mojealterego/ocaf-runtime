# Katalog komponentów OCAF / OMAs / OMAt / OSTc

Ten katalog jest wspólnym punktem wejścia dla operatora i recenzenta. Opisuje granice, kontrakty i aktualne dowody bez utożsamiania planu z wdrożeniem.

## OCAF Runtime

OCAF Runtime jest warstwą kontraktów i wykonania agenta: identyfikacja, manifest, lifecycle, limity, wejścia/wyjścia, logowanie i dowody. Runtime nie nadaje sam uprawnień do narzędzi oraz nie może omijać blokady OSTc. Agent przechodzi przez `DRAFT`, `VALIDATED`, `BUILT`, `TESTED`, `SIGNED`, `REGISTERED`, `STAGED`, a dopiero potem może żądać `ACTIVE`.

**Stan:** PARTIAL. **Brakujące dowody:** kompletny artefakt referencyjny, podpis i zewnętrzny test E2E.

## OMAs — Agent Server

OMAs jest osobnym serwerem agentów. Odpowiada za rejestr, admisję, sesję i kontrolę tożsamości agenta. Nie jest serwerem narzędzi. Każde żądanie powinno mieć identyfikator agenta, wersję manifestu, tenant, korelację, zakres i dowód integralności.

**Stan:** PARTIAL. **Brakujące dowody:** niezależny proces OMAs z testem authn/authz, izolacją tenantów i odtwarzaniem błędu.

## OMAt — Tool Server

OMAt jest osobnym serwerem narzędzi. Rejestruje narzędzie, wersję, zakres uprawnień, wejścia/wyjścia, limity i politykę wykonania. OCAF/OMAs mogą otrzymać wyłącznie narzędzie przyznane w manifestach i potwierdzone polityką.

**Stan:** PARTIAL. **Brakujące dowody:** katalog narzędzi z izolowanym executor-em i testem odrzucającym nieuprawnione wywołanie.

## Zenoh — transport

Zenoh przenosi komunikaty i synchronizację między usługami. Transport nie jest autoryzacją: pochodzenie, zakres i tenant muszą być sprawdzone przez kontrakty OMAs/OMAt. Timeout, duplikat i komunikat spoza sesji muszą kończyć się błędem zamkniętym.

**Stan:** PARTIAL. **Brakujące dowody:** odtwarzalny test wielowęzłowy z utratą, duplikatem i opóźnieniem komunikatu.

## OSTc — przyszła ochrona źródeł

OSTc ma dostarczyć szyfrowany, kontrolowany dostęp agenta do jego pliku źródłowego. W tym wydaniu OSTc jest **wyłączone**. Nie ma podstaw do twierdzenia, że źródła są zaszyfrowane. Z tego powodu agent nie uzyskuje dostępu do źródła, a próba przejścia do `ACTIVE` jest odrzucana niezależnie od deklarowanego potwierdzenia gotowości.

**Stan:** DISABLED / FUTURE. **Warunek uruchomienia:** pełna specyfikacja, implementacja, testy kryptograficzne, rotacja kluczy, odzyskiwanie i recenzja.

## Statusy i materiały marketingowe

`IMPLEMENTED_VERIFIED` oznacza dowód kodu, artefaktu, SHA-256, commita i testów. Broszura, plakat, wykres, wycena lub rekord autentyczności nie podnosi statusu technicznego i nie jest certyfikatem prawnym.

## Dokumentacja serwisowo-techniczna

### OCAF Runtime — instalacja i obsługa

1. Sprawdź toolchain Rust, `Cargo.lock` i lokalne zależności.
2. Uruchom `make agent001-test` dla pakietu referencyjnego.
3. Zbuduj przez `make agent001-build`, a instalator uruchom po przeglądzie
   ścieżki docelowej: `OCAF_AGENT_PREFIX=/var/lib/ocaf-agent-001 make agent001-package`.
4. Zachowaj checksum binarki, manifest, commit i log testów w rejestrze dowodów.

Runtime zarządza manifestem, lifecycle, limitami, korelacją i dowodami. Nie
przyznaje samodzielnie dostępu do źródła i nie włącza OSTc. Awaria storage,
transportu, podpisu lub limitu kończy się odmową i zachowaniem poprzedniego stanu.

### OMAs — serwer agentów

OMAs przyjmuje wyłącznie żądania z `agent_id`, tenantem, wersją manifestu,
korelacją, capability i digestem artefaktu. Admisja sprawdza tożsamość,
tenant boundary, podpis, replay nonce, limity, uprawnienia i lifecycle. Błąd
dowolnego kroku oznacza `DENIED`; efekt uboczny nie może wystąpić wcześniej.

Runbook OMAs: sprawdź zdrowie procesu i zegar, epoch/fencing, tenant namespace
i wygasłe leases; przed zmianą konfiguracji wykonaj test podpisu, replay i
cross-tenant. Przy awarii zatrzymaj nowe żądania, zachowaj dziennik i wznowienie
wykonaj dopiero po kontroli spójności. OMAs nie jest OMAt.

### OMAt — serwer narzędzi

OMAt prowadzi osobny rejestr narzędzi. Wpis wymaga identyfikatora, wersji,
schematu wejścia/wyjścia, capability, limitów CPU/czasu/rozmiaru, retry policy,
tenant scope i klasy ryzyka. Wywołanie jest dozwolone tylko, gdy allowlist
manifestu obejmuje dokładnie wersję narzędzia.

Runbook OMAt: waliduj schema i rozmiar; zwiąż próbę z correlation ID, tenantem,
agentem i digestem; wykonuj w izolowanym executorze; zapisz receipt albo
`UNCERTAIN`; nie ponawiaj efektu ubocznego bez idempotency key. Liczba narzędzi
nie może być deklarowana bez inwentaryzacji, licencji i testów.

### Zenoh — transport

Zenoh przenosi komunikaty między agentem, OMAs i OMAt, ale nie nadaje praw.
Konfiguracja musi mieć endpoint, TLS/mTLS lub równoważną autoryzację,
namespace tenantowy, limity ramek, timeout i reconnect. Komunikat spoza
namespace, po terminie lub z powtórzonym nonce jest odrzucany. Test obejmuje
opóźnienie, duplikat, utratę i zmianę epoch.

### OSTc — procedura przyszłej integracji

OSTc jest obecnie `DISABLED`. Dostęp agenta do źródeł pozostaje wyłączony,
a `ACTIVE` jest odrzucane także przy fałszywej gotowości. Przed włączeniem
trzeba dostarczyć protokół, model zagrożeń, parametry, rotację i odzyskiwanie
kluczy, wektory testowe, audyt oraz adapter manifestu. Nie wolno nazywać źródła
zaszyfrowanym ani zastępować OSTc deklaracją konfiguracji.

## Weryfikacja wydania i troubleshooting

`STAGED` wymaga source commit, toolchain, manifestu, digestu artefaktu, testów
pozytywnych i negatywnych, logu instalacji oraz rollbacku. `ACTIVE` wymaga
dodatkowo podpisu, gotowości OMAs/OMAt, aktywnego OSTc i przyznanego dostępu.
`declared_state` nigdy nie jest dowodem.

| Objaw | Sprawdzenie | Działanie |
|---|---|---|
| `OstcDisabled` | konfiguracja OSTc | pozostaw `STAGED`; nie obchodź bramki |
| `SourceAccessDenied` | przyznanie przez OSTc | przywróć stan wyłączony |
| `ArtifactMismatch` | SHA-256 binarki i manifest | przebuduj i zarejestruj digest |
| `CommitMismatch` | commit źródłowy | zatrzymaj wydanie i odtwórz build |
| timeout OMAs/OMAt | endpoint, epoch, lease | odmów, zachowaj receipt, usuń lease |
| replay / nieznane narzędzie | nonce i rejestr OMAt | odrzuć; nie ponawiaj efektu |

Każda naprawa wymaga nowego dowodu. Ręczna zmiana statusu bez dowodu jest
naruszeniem kontraktu.
