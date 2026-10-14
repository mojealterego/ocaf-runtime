# OCAF 0.38 — rejestr zewnętrznych odbiorów (A01–A12)

**Status: NO-GO; otwarty P0.** `main` wyłącznie. Plik jest rejestrem
brakujących dowodów, **nie** poświadczeniem ich wykonania. Funkcja Issues
jest wyłączona w repozytorium (GitHub HTTP 410); dlatego utrzymywać
rejestr zmian w tym dokumencie i w nadrzędnej roadmapie.

## Pakiet 1: dwa trwałe hosty, dwaj tenanty, chaos i odtwarzanie

- Wymagane: `host_a_systemd`, `host_b_systemd`,
  `multi_host_mtls_rpc`, `two_tenant_isolation`,
  `network_partition_recovery`, `sigkill_replay_fencing`,
  `publisher_tls_revocation`, `soak_100_cycles`,
  `supply_chain_provenance`, `backup_restore`.
- Dwa odrębnie administrowane trwałe systemy Linux x86_64,
  systemd jako PID1, cgroup v2, odrębne failure domains.
- Dwa rzeczywiste tenanty jednocześnie na dwóch maszynach,
  operatorowa PKI/mTLS i Ed25519, zatwierdzony pinned Wasmtime.
- Odmowy cross-principal, usunięcie zdublowanych effect writes
  po network partition i SIGKILL, rotacja CA oraz podpisu wydawcy,
  pomiary cgroup oraz jeden 100-cyklowy soak na **każdym** hoście.
- Test tylko na izolowanym staging z odłączonymi rzeczywistymi
  skutkami zewnętrznymi; raporty prywatne, sha256 w publicznym opisie.
- Bez kupowania serwerów, wdrażania płatnych usług lub wystawiania
  sekretów. Potrzebne wcześniej uprawnione zasoby i plan odzyskiwania.
- Stan: **nie potwierdzono trwałych dwóch hostów**, CI ma test
  cross-VM jednego tenant-a i test 2 tenantów na jednej maszynie.

## Pakiet 2: niezależny audyt rzeczywistej osoby/podmiotu

- Wymagany: `independent_security_audit`.
- Audytor *nie* jest członkiem zespołu implementacyjnego ani
  agentem/skanerem zastępczym.
- Zakres: native Wasmtime, sandbox, TOCTOU, mTLS leaf PoP,
  signed manifests, replay/fencing, host privileges, CVE/SBOM,
  realne artefakty pakietu 1, negatywne próby i residual risk.
- Dokument: raport, reprodukowalne findings, finalny SHA,
  tabela kontroli, hash każdego załącznika, tożsamość audytora,
  osobny prawidłowy podpis Ed25519.
- Wyznaczony przez użytkownika kandydat: **zchiz444**. Jego niezależność i tożsamość **nie zostały zweryfikowane**, a rzeczywisty audyt i podpisany raport A11 nadal nie istnieją.

## Pakiet 3: operator i formalny odbiór change-control

- Wymagany: `operator_signoff`.
- Uprawniony operator osobiście weryfikuje dowody dwóch hostów,
  finalny SHA, SLO/RTO/RPO, rollback i raport audytora.
- Podejmuje decyzję ACCEPT / REJECT i podpisuje osobnym kluczem
  Ed25519; publiczne klucze rol nie mogą pochodzić z katalogu
  kontrolowanych dowodów.
- Finalny pakiet testować przez `scripts/stage038_production_gate.py`.
  Kod zwraca `production_approved: false` nawet przy poprawności
  kryptograficznej, gdyż wymagane jest niezależne zatwierdzenie
  release/change-control poza skryptem.
- Operator wskazany przez użytkownika: **Andrzej Mikulski**. Brak podpisanej decyzji A12 po przeglądzie A01–A11; wpisanie nazwiska samo nie oznacza ACCEPT.

## Zweryfikowana baza CI

Dla commit `361fefece39ed0f34675c8e3d0fd5f8054da41e0`:
Rust CI 38618072351, CodeQL 38618072199, Supply Chain 38618072116,
host 38618072200, 2-host 38618072312, dual-tenant 38618072122
i 100-cycle *CI sharded* soak 38618072521 zakończyły się SUCCESS.
To **historyczny SHA** — po aktualizacji `main` sprawdzić nowy zestaw.

Automatyczny skan zizmor (zewnętrzne narzędzie, **nie niezależny
audytor**) ujawnił problemy w definicjach workflow. Na oddzielnym
lokalnym checkout poprawiono referencje akcji, digest obrazu, szeroki
OIDC i przechowywanie credential checkout; 148 Python unit tests
PASS, zizmor offline regular 0 findings. Zmiany przenoszono do
`main` sekwencyjnie. Zielone CI na wcześniejszych commitach **nie
może** zatwierdzić kolejnego, zmienionego SHA.

## Decyzja

**0.38 PARTIAL / NO-GO; 0.39 zablokowany.** Nie fałszować dowodów,
tożsamości, czasu, podpisów ani decyzji. Zewnętrzne rzeczywiste
podpisanie kończy gate; przygotowanie dokumentów nie zastępuje testu.

Odwołania:
[runbook](STAGE038_EXTERNAL_ACCEPTANCE_RUNBOOK.md),
[audyt](STAGE038_INDEPENDENT_AUDIT_HANDOFF.md),
[operator](STAGE_038_OPERATOR_SIGNOFF.md),
[roadmap](ROADMAP_FULL_ECOSYSTEM_2026.md).

## Przypisanie osób — 2026-10-15

| Kontrola | Wyznaczona nazwa | Status |
| --- | --- | --- |
| A11 — niezależny audyt bezpieczeństwa | **zchiz444** | Wskazany; niezależność i akceptacja roli niezweryfikowane; brak wyników i podpisu |
| A12 — odbiór przez operatora | **Andrzej Mikulski** | Wskazany; nie zatwierdził zweryfikowanego pakietu, podpis końcowy nie istnieje |

Rejestr trzech wcześniejszych technicznych podpisów inicjalizacyjnych jest
oddzielny od podpisów pod końcowym pakietem A11/A12.
**Status nadal P0 / NO-GO** — nie fabrykować dowodów ani dat zamknięcia.
