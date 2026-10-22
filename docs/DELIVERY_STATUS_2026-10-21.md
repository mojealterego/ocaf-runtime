# Stan dostawy OCAF — 2026-10-21

## Zakres dostarczony

- Rejestr organizacyjny: **33 roje / 111 osobnych agentów**, wszystkie wpisy `NOT_DEPLOYED`;
  aktywnych agentów runtime: `0`.
- Fizyczne pakiety floty: [`AGENT-FLEET-111`](<../OCAF Agents MOJEALTEREGO/AGENT-FLEET-111/>);
  każdy z 111 katalogów ma własną strukturę i manifest.
- Flota rojów: [`SWARM-FLEET-33`](<../OCAF Agents MOJEALTEREGO/SWARM-FLEET-33/>);
  33 osobne roje mają własny skład, handoff, etapowanie i healthcheck.
- Pakiet referencyjny `OCAF-AGENT-001`: zbudowany i przetestowany lokalnie,
  lifecycle `STAGED`, instalator POSIX, SHA-256 zapisany w
  `OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/packaging/BUILD_EVIDENCE.md`.
- Kontrakty instalacyjne wszystkich 10 wariantów Agents Builder (Cloud,
  Windows, macOS, Android, iOS × Code/NoCode), z osobnym wymaganiem artefaktu,
  podpisu, testu platformowego i rollbacku.
- Inwentaryzacja bibliotek i infrastruktury: DGM/RSI/RQGM, AST/IR,
  OmniMath/SMT/ImandraX, Firecracker/KVM, eBPF/Aya, Zenoh, MCP i L402.
- Kontrakty usług: [OMAs](OMAS_SERVER.md), [OMAt](OMAT_SERVER.md), [Zenoh](ZENOH_AGENT_PLANE.md)
  oraz [infrastruktura systemowa](infrastructure/system-infrastructure-v1.json).
- Odzyskiwanie roadmapy: [ROADMAP_RECOVERY.md](ROADMAP_RECOVERY.md), trzy
  niezależne snapshoty SHA-256 i skrypt wielokrotnej budowy.
- Dzisiejszy postęp implementacyjny: stage 0.39.8 lifecycle contract, ordered
  stage ledger oraz stage 0.40 independent-host HA contract. Realne KVM/hosty
  pozostają wymagane do `COMPLETE_VERIFIED`.
- Pipeline dowodowy: [EVIDENCE_PIPELINE.md](EVIDENCE_PIPELINE.md); bieżący
  baseline ma status `PENDING_EVIDENCE`.
- Supervisor floty: [AGENT_FLEET_SUPERVISOR.md](AGENT_FLEET_SUPERVISOR.md),
  waliduje 111 agentów i 33 roje oraz zapisuje audyt JSONL.
- Dokumentacja serwisowa, techniczna, operatorska, weryfikacyjna oraz materiały
  broszurowe, wykresy, plakat i robocza wycena.

## Granice architektury

OMAs pozostaje osobnym serwerem agentów, OMAt osobnym serwerem narzędzi, a
Zenoh warstwą transportową. OSTc jest przyszłą warstwą szyfrowanego dostępu do
źródeł i w tym wydaniu pozostaje `DISABLED`. Przy tej konfiguracji `ACTIVE` jest
odrzucane także przy fałszywym readiness.

## Co nadal wymaga środowiska docelowego

Nie utworzono fikcyjnych MSI/DMG/PKG/APK/AAB/IPA ani obrazu cloud. Osiem
kontraktów ma status `BLOCKED_PLATFORM_BUILD`, a dwa `NOT_IMPLEMENTED`, dopóki
nie będzie odpowiedniego runnera, kodu produktu, podpisu i testu E2E. OMAs,
OMAt, OSTc, DGM i RSI pozostają odpowiednio `PARTIAL` albo `PLANNED` zgodnie z
inwentaryzacją; nazwa w roadmapie nie jest dowodem wdrożenia.

## Dowody

- `cargo test --workspace --no-default-features --lib --tests`: 559 testów,
  0 błędów w zapisanym przebiegu.
- Testy pakietu referencyjnego: 2 integration + 4 security, wszystkie zaliczone.
- Rejestr maszynowy potwierdza 33 roje, 111 osobnych agentów i 0 aktywnych runtime.
- `git diff --check`: przechodzi.
