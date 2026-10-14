# OCAF 0.38 — niezależny audyt bezpieczeństwa: pakiet do przekazania / NO-GO

**Stan: zchiz444 wskazany przez użytkownika jako kandydat na niezależnego audytora. NIE zweryfikowano tożsamości, niezależności ani uprawnień; brak przeprowadzonego audytu, raportu i podpisu.** Ten dokument jest zakresem zlecenia weryfikacyjnego, a nie rzekomym raportem audytora. Brak opłat i jakichkolwiek nowych zobowiązań finansowych.

## Granica odbioru

Przedstawiciela spoza zespołu implementacyjnego nie wolno zastępować autorem kodu, CodeQL, sztuczną recenzją ani CI. Audytor musi mieć odrębną tożsamość i podpis Ed25519; operator zatwierdza ryzyko *osobnym* podpisem pod tym samym aktualnym SHA main. Weryfikator `scripts/stage038_production_gate.py` nadal zwraca `production_approved: false` nawet po kryptograficznej poprawności pakietu.

## Zestaw wejściowy

- Repo: https://github.com/mojealterego/ocaf-runtime; branch: **main wyłącznie**.
- Zagrożenia i kontrakty: `docs/SECURITY.md`, `docs/WASI_EXECUTION_038.md`, `docs/PRIORITY_038_EXECUTION_CONTRACT.md`, `docs/STAGE_038_OPERATOR_SIGNOFF.md`.
- Silnik wykonawczy: `src/executor/wasi.rs`, `src/executor/wasi_manifest.rs`, `src/executor/wasi_receipt.rs`, `src/ops/wasi_config.rs`.
- Transport/polityka: `src/transport/tls_bound.rs`, `src/rpc/mod.rs`, `src/bin/ocaf-coordinator.rs`, `src/runtime/mod.rs`.
- Dwa niezależne GitHub Linux VM: workflow `.github/workflows/stage038-twohost.yml`, sukces run 38614920507 dla SHA e442734f27e49e640941b77eee7f11332cd180f3.
- Dwa oddzielne tenanty z systemd, TLS/mTLS i Wasmtime na jednym efemerycznym Linux runnerze: workflow `.github/workflows/stage038-multitenant-chaos-ci.yml`, sukces run 38617180984 dla SHA 19c639eb10ca42f778eb9f58ef55a347757c59b2.
- Wszystkie raporty CI są dowodami mechanizmu, **nie** dowodami trwałego staging, operator signoff ani certyfikacją.

## Zakres kontroli audytora

1. Oceniać adwersarza z pełną kontrolą nad modułem WASIp1, wejściem JSON, stdout/stderr, próbami nadużycia hostowych FD/FS/CPU/PID, delegacją do Wasmtime i interakcjami z TLS/RPC.
2. Przeprowadzić niezależne code review polityki manifestów Ed25519, digestów, ekskluzywnego tenant:agent, dat ważności, ochrony fd/inode, memfd/seals, procesu potomnego i sprzątania grupy/cgroup.
3. Zweryfikować mTLS cert chain, leaf pin, CN/SAN, PoP, ACL, replay, rotację i cofnięcie zaufania na działających **trwałych** węzłach.
4. Rozdzielić długotrwałą współbieżność tenantów i niepewne efekty od syntetycznych no-op fixtures. Przeprowadzić awarie hosta, wyłączenie procesu, zerwanie sieci między oddzielnymi domenami awarii, restart, powrót, backup/restore i fencing bez duplikacji zewnętrznych skutków.
5. Zmierzyć rzeczywiste ograniczenia memory.peak, cpu.stat, pids.peak, nie tylko RSS próbkowane przez proces, oraz odtworzyć błędy przy skrajnych limitach.
6. Przejrzeć supply chain, SBOM, advisories RustSec i silnika, CodeQL oraz konfigurację powierzchni hosta; dowieść zamknięcia znalezionych podatności na *końcowym SHA*.
7. Przekazać podpisany raport z zakresem, wynikiem, powtarzalnymi ustaleniami, ryzykiem rezydualnym, datą, SHA, tożsamością audytora, decyzją i listą załączników SHA-256.

## Warunek formalnego odbioru

Operator powinien niezależnie podpisać kompletny wynik po obejrzeniu artefaktów z rzeczywistych trwałych hostów. Wymagane dowody: `host_a_systemd`, `host_b_systemd`, `multi_host_mtls_rpc`, `two_tenant_isolation`, `network_partition_recovery`, `sigkill_replay_fencing`, `publisher_tls_revocation`, `soak_100_cycles`, `supply_chain_provenance`, `backup_restore`, `independent_security_audit`, `operator_signoff`.

Weryfikacja:

```bash
python3 scripts/stage038_production_gate.py \
  --directory /secure/stage038-acceptance \
  --expected-commit "$(git rev-parse main)" \
  --operator-public-key /secure/trust/operator.pem \
  --auditor-public-key /secure/trust/auditor.pem
```

NIE tworzyć podpisów w imieniu innych osób. Nie kopiować kluczy prywatnych do czatu ani GitHub. Nie oznaczać etapu 0.38 jako ukończonego, dopóki odbiór nie jest rzeczywiście podpisany i zweryfikowany. Przedtem bezwarunkowo `NO-GO`. Etap 0.39 pozostaje zablokowany.

## Imienne wskazanie audytora — 2026-10-15

- **Audytor wskazany do roli A11:** `zchiz444` (nazwa przekazana przez użytkownika).
- **Status wyznaczenia:** wskazanie administracyjne; niezależność, rzeczywista tożsamość, kompetencje i osobiste przyjęcie roli **niezweryfikowane**.
- **Wymagane działanie:** `zchiz444` samodzielnie przeprowadza kontrolę kodu i dowodów A01–A10, dokumentuje rzeczywiste wyniki PASS/FAIL, podpisuje raport własnym kluczem pod wyłączną kontrolą i przekazuje do operatora.
- **Niedozwolony substytut:** podpis wygenerowany przez asystenta dla tej nazwy pod deklaracją zgody nie jest niezależnym podpisem audytora i nie stanowi akceptacji A11.
- **Operator wskazany do roli A12:** Andrzej Mikulski. Przegląd i końcowy podpis dopiero po kompletnym raporcie A11.

**Stan A11: PENDING / NO-GO.**
