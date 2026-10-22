# Runbook techniczny infrastruktury agentowej — 2026-10-22

Dokument opisuje wyłącznie kontrakty i pliki obecne w repozytorium. Status
`PARTIAL` oznacza kod kontraktowy z otwartym dowodem E2E. Nie oznacza działającej
usługi produkcyjnej.

## 1. OCAF Runtime i lifecycle

Źródła: `src/agent_fleet.rs`, `src/stage_ledger.rs`, `src/security_gate.rs`,
`src/runtime/`, `scripts/ocaf_agent_fleet_supervisor.py`.

Supervisor sprawdza 111 manifestów agentów i 33 manifesty rojów. Etapowy ledger
wymaga kolejności zależności, commita, wyniku testu, digestu artefaktu i
`receipt_id`. Brak któregoś pola pozostawia etap otwarty.

```sh
python3 scripts/ocaf_agent_fleet_supervisor.py --log /tmp/ocaf-supervisor.jsonl
python3 scripts/evidence_pipeline.py collect --output /tmp/fleet-baseline.json
python3 scripts/evidence_pipeline.py validate --output /tmp/fleet-baseline.json
```

## 2. OMAs — serwer agentów

Źródła: `src/omas_server.rs`, `src/omas_policy.rs`, `src/omas_signed.rs`,
`src/bin/ocaf-omas-server.rs`, `docs/OMAS_SERVER.md`.

OMAs waliduje identyfikator agenta, tenant, digest manifestu, replay request ID
i stan `STAGED`. Nie uruchamia narzędzi. Przy `ostc_enabled=true`,
`source_access != disabled`, `ACTIVE` albo fałszywym readiness odmawia.

```sh
cargo run --no-default-features --bin ocaf-omas-server -- --health
```

Polecenie wymaga dostępnego toolchainu Rust. Kod obecny w repozytorium nie jest
dowodem działającego endpointu sieciowego, trwałego rejestru ani E2E tenantów.

## 3. OMAt — serwer narzędzi

Źródła: `src/omat_server.rs`, `src/registry/`, `src/executor/`,
`docs/OMAT_SERVER.md`, `docs/OMAT_SERVICE_UNIT.example`.

OMAt rejestruje manifest narzędzia, schemat JSON, tenant, capabilities, limity
i ryzyko efektu ubocznego. `admit` tworzy bounded receipt. `execute` pozostaje
fail-closed z `ExecutorUnavailable`, dopóki osobny executor nie ma własnego
digestu i receiptu.

Binarny health adapter:

```sh
cargo run --no-default-features --bin ocaf-omat-server
```

Nie wolno traktować katalogu narzędzi jako wykonania kodu ani uruchamiać
zewnętrznych efektów przez sam manifest.

## 4. Zenoh — warstwa transportowa

Źródła: `deploy/zenoh-plane-contract.json`,
`deploy/zenoh-ocaf-omas-omat.json5`, `scripts/zenoh_plane_contract.py`,
`docs/ZENOH_AGENT_PLANE.md`.

Kontrakt dopuszcza ścieżki OCAF↔OMAs i OMAs↔OMAt, zabrania bezpośredniej trasy
OCAF→OMAt, wymaga tenant namespace, replay store, health dependencies i TLS/mTLS.
Zenoh nie nadaje uprawnień.

```sh
python3 scripts/zenoh_plane_contract.py --contract deploy/zenoh-plane-contract.json
```

To jest walidacja kontraktu, nie uruchomienie routera ani test wielowęzłowy.

## 5. DGM / RSI / RQGM

### RSI i kontrolowana ewolucja

Źródła: `src/proof_carrying_rsi.rs`, `src/evolve_candidate.rs`,
`src/omni_verify.rs`, `src/omni_proof_stark.rs`.

Obecny zakres obejmuje bounded proof checker, candidate lease, evaluation
receipt, approval dla celu production oraz brak hot swap. Moduły zwracają
autoryzację jako dane; nie mutują produkcji i nie uruchamiają kodu kandydata.

### DGM / RQGM

Roadmapa opisuje DGM/RQGM jako wymaganie badawcze. W repozytorium są kontrakty
proof-carrying i kontrola epoki, ale nie ma pełnej Darwinian Gödel Machine,
autonomicznej pętli research, produkcyjnego hot swap ani niezależnego backendu
Lean/SMT/ImandraX. Workspace odwołuje się do `crates/ocaf-cognition-rqgm`, lecz
lokalny cel symlinkowy `crates -> /tmp/ocaf-crates` jest obecnie niedostępny;
nie wolno raportować tego crate jako zbudowanego.

## 6. Kolejność odbioru

1. Zweryfikować integralność roadmapy: `python3 scripts/roadmap_integrity_gate.py`.
2. Zweryfikować supervisor floty.
3. Zbudować OMAs i OMAt osobno.
4. Uruchomić walidację Zenoh.
5. Wykonać testy pozytywne i negatywne tenant/replay/capability.
6. Dopiero wtedy dołączać backendy Firecracker/eBPF lub zewnętrzne solvery.
7. OSTc pozostaje wyłączone; `ACTIVE` pozostaje odrzucone.

Każdy wynik musi zawierać repozytorium, commit, komendę, rezultat, SHA-256 i
receipt. Deklaracja gotowości bez tych pól jest odrzucana.
