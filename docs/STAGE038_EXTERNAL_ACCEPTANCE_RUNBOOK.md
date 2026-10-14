# OCAF 0.38 — rzeczywisty odbiór: dwie maszyny, audyt, podpisy

**Status (2026-10-14): production NO-GO.** Niniejszy runbook nie jest dowodem
przeprowadzenia testów ani niezależną opinią audytora. Pracować wyłącznie na
gałęzi `main`, ale **raportów hostowych, sekretów i prywatnych podpisów nie
publikować w publicznym repozytorium**.

## Środowisko konieczne do wykonania

- Dwa **rzeczywiste, odrębnie administrowane** hosty Linux x86_64 z PID 1
  `systemd`, cgroup v2, pinned Wasmtime 49.0.2 i wdrożonym OCAF. Dla każdego
  potwierdzić osobny `/etc/machine-id`, VM/hypervisor ID, failure domain i
  kontrolowany kanał administracyjny; dwa procesy/kontenery na jednym hoście
  nie spełniają warunku.
- Wydzielony, niedostępny publicznie staging; dwa syntetyczne tenanty A i B,
  dwa certyfikaty klienta mTLS oraz administrator PKI/KMS.
- Niezależny audytor bezpieczeństwa, inna osoba/podmiot niż wykonawca kodu,
  oraz operator upoważniony do decyzji o produkcyjnym wdrożeniu; oddzielne
  klucze Ed25519 i niezależnie zatwierdzona lista ich kluczy publicznych.
- Zatwierdzone scenariusze awaryjne, ograniczenia testów i plan odzyskiwania.
  Testy `SIGKILL`, network partition i wyczerpania zasobów **tylko w staging**.

**Weryfikacja stanu bieżącego:** repozytorium zawiera wyniki tylko z
efemerycznych runnerów GitHub i lokalnych procesów. Nie ma w repozytorium
autorytatywnego poświadczenia dwóch działających docelowych hostów ani
niezależnego podpisu. Żadnego wyniku CI nie wolno awansować do GO.

## Realna sekwencja odbiorowa

1. **Pin rewizji.** Operator identyfikuje jedno `git rev-parse HEAD`, sprawdza
   CI Rust, CodeQL, Supply Chain, podpisy paczek i hash zatwierdzonego
   Wasmtime. Późniejsza zmiana kodu unieważnia dowody dla starego SHA.
2. **Dwa hosty.** Na A i B osobno uruchamia
   `python3 scripts/stage038_host_gate.py --unit ocaf-coordinator-wasi.service
   --report /var/lib/ocaf/evidence/host-038.json`. Raporty trzymać w
   prywatnym katalogu, bez nadpisywania i bez ujawniania kluczy. Dowody
   obejmują `memory.peak`, `pids.current`, `cpu.stat`, `memory.events`,
   właściciela procesu, limit memory/pids/cpu i `NoNewPrivileges`.
3. **Prawdziwa sieć.** Uruchamia OCAF RPC/TLS między hostami A/B z
   wzajemną walidacją tożsamości certyfikatów. Sprawdza osobno pozytywne
   wywołania zatwierdzonych narzędzi WASI tenantów A/B, odmowę podpisu
   innego wydawcy, odmowę cross-tenant/cross-agent, negatywne przypadki
   `replay` i rozdzielenie logów/receiptów po obu stronach.
   **Handshakes wyłącznie po loopback na jednym runnerze nie wystarczają.**
4. **Chaos.** Na dedykowanym staging wymusza utratę łączności A/B,
   `SIGKILL` koordynatora, restart systemd, utratę uprawnionego lease,
   presję pamięci i CPU, timeout i odzyskanie po partycji. Raportuje
   fencing epok, liczbę `uncertain` efektów, brak automatycznego
   powtórzenia niepewnego skutku i cleanup cgroup/PrivateTmp.
5. **Trwałość i rotacja.** Przywraca PostgreSQL i niezależne dzienniki
   sinków z kopii testowej, rozlicza `uncertain`, wymienia CA/certyfikaty
   klientów i klucz wydawcy Ed25519, sprawdza natychmiastową odmowę dla
   odwołanych poświadczeń po zatwierdzonym restarcie procesów.
6. **Soak.** Każdy host przechodzi zatwierdzony przez operatora próg
   stabilności; jeden ciąg 100 cykli rzeczywistego Wasmtime i równoległe
   obciążenie dwóch tenantów. Artefakt z
   `scripts/wasi_host_acceptance.py` zestawić z autorytatywnym
   `memory.peak` i budżetem SLO. Pięć runnerów CI po 20 cykli to nie ten
   sam test.
7. **Niezależny audyt.** Audytor poza zespołem implementacyjnym weryfikuje
   kod, build/provenance, Wasmtime native attack surface, mTLS-PoP,
   tenant/agent ACL, podpisy, revocation, replay/fencing, izolację procesu
   i uprawnienia systemd; wykonuje własne próby negatywne. Wystawia
   podpisany raport z zakresem, metodą, CVE/findings, ryzykiem
   resztkowym oraz decyzją PASS/FAIL; nie można zastąpić go CodeQL.
8. **Formalny odbiór.** Operator sprawdza autentyczność raportów hostowych
   i podpisanego raportu audytora, zatwierdza SLO, limity i plan rollback,
   a następnie składa **własny podpis** zewnętrznym kluczem. Wniosek bez
   podpisów pozostaje NO-GO.

## Maszynowo sprawdzalny pakiet odbioru

W chronionym folderze dowodów utworzyć `acceptance.json`, bez prywatnych
kluczy i bez danych tenantów. Dokładny schemat w
`scripts/stage038_production_gate.py`:

- `schema_version=1`, `git_commit` = 40-znakowy SHA `main`,
  `observed_at_utc` (UTC `Z`, wiek ≤30 dni), `decision="ACCEPT"`.
- `hosts`: dwa obiekty o rolach `coordinator` i `worker`, każdy zawiera
  `host_id`, `machine_id_sha256`, `failure_domain`, `role`,
  `systemd_pid1=true`, `cgroup_v2=true`, `ephemeral_ci=false`.
  Wartości IDs i failure domains muszą być odrębne; niezależny audyt
  fizycznych maszyn nadal jest obowiązkowy.
- `assertions`: wszystkie osiem pozytywnych twierdzeń z
  `REQUIRED_ASSERTIONS`, każde równe `true`, **dopiero po wykonaniu**.
- `evidence`: dokładnie 12 pozycji `REQUIRED_EVIDENCE` w skrypcie,
  każda `{"file":"nazwa.json","sha256":"64_znaki_hex"}`, bez symlinków,
  dodatkowych katalogów, duplikatów ani pustych plików.

Podpisy Ed25519 obejmują **identyczne bajty** `acceptance.json`, z oddzielną
separacją domen na rolę:
`b"OCAF038-ACCEPTANCE-V1\0" + role.encode("ascii") + b"\0" + raw_json`.
Po podpisaniu przez operatora i audytora odrębnie, binarne pliki
`operator.sig` i `auditor.sig` muszą leżeć obok `acceptance.json`.
Zatwierdzone klucze publiczne `operator.pem`, `auditor.pem` należy trzymać
**poza folderem nieufnych dowodów**. Kluczy prywatnych nigdy nie umieszczać
w GitHub, artefaktach Actions ani w treści rozmowy.

Zewnętrzny weryfikator:

```bash
python3 scripts/stage038_production_gate.py \
  --directory /secure/ocaf038-evidence \
  --expected-commit "$(git rev-parse HEAD)" \
  --operator-public-key /secure/trusted/operator.pem \
  --auditor-public-key /secure/trusted/auditor.pem
```

Kody wyjścia: `2` = NO-GO (dowody niepełne/fałszywe/przeterminowane/
podpisy nieważne), `0` = poprawna struktura, SHA-256 i dwa prawidłowe
podpisy, **nadal wymagające ludzkiej weryfikacji i zmiany statusu poza
skryptem**. W obu przypadkach zwracane `production_approved=false`:
mechaniczny weryfikator nie może sam wydać produkcyjnego GO.

## Krytyczna uwaga do CI

Sukces `OCAF 0.38 Signed WASI systemd TLS staging probe` oznacza lokalny
probe na efemerycznym runnerze i **nie jest** testem wielohostowym.
Podobnie raport bezpieczeństwa z CodeQL jest analizą automatyczną,
a nie niezależnym poświadczeniem audytowym. Zmiana statusu produkcyjnego
bez rzeczywistej infrastruktury i rzeczywistych podpisów byłaby
fałszowaniem procesu odbiorowego.
