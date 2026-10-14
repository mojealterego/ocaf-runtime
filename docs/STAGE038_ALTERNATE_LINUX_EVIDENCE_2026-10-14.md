# OCAF 0.38 — alternatywny Linux (Sprite), dowody częściowe / NO-GO

Data: 2026-10-14. Ten zapis dokumentuje tylko rzeczywiste wyniki diagnostyki; **nie jest zatwierdzeniem produkcyjnym ani etapem 0.39**.

## Źródło

- Środowisko: Sprite `ocaf-stage038-linux`, Ubuntu 26.04, Linux x86_64, kernel `6.12.105-fly`.
- Repozytorium: `mojealterego/ocaf-runtime`, git SHA `42b2fe3cdc2c14198f81ffa5de2c36d8247607a1`.
- Silnik: oficjalne Wasmtime `49.0.2`, zweryfikowany upstream archive SHA-256 przez `scripts/wasmtime_acceptance.sh`.
- Hash użytego binarium silnika: `d0a014e0d5b0cf48dd3549c38e2e6ecd78ff09fb3e3f10d751b87b72bdfd8635`.
- Lokalne dowody: `/home/sprite/stage038-wasmtime-run.log`, `/home/sprite/stage038-wasi-10cycle.json`, `/home/sprite/stage038-host-sprite.json`. Przechowywane na Sprite, bez eksportowania tajnych danych.

## Zaobserwowane wyniki

- Test integracyjny `wasi_sandbox` przy prawdziwym Wasmtime: **17/17 PASS**.
- Soak: **10/10 cykli PASS**. Limity: RSS 1 572 864 KiB; FD 512; PID 64. Zaobserwowane najwyższe wartości: RSS 183 128 KiB, FD 22, procesów 4. Brak nowych prywatnych katalogów tymczasowych po próbach.
- Host gate: `linux_x86_64=true`, `cgroup_v2_cpu_memory_pids=true`, `memfd_sealed=true`, `pdeathsig_supported=true`, `proc_fd=true`.
- **Blokada:** `systemd_pid1=false`, `systemctl_installed=false`, PID 1 to `tini`. Wymaganego izolowanego serwera staging z jednostką systemd nie zastępuje samo `cgroup2fs`.

## Następny kontrolowany krok

Ponownie uruchomić już istniejący pięcioszardowy workflow `stage038-extended-soak.yml` (5 × 20 rzeczywistych cykli Wasmtime) na *końcowym SHA main*. Każdy shard zapisuje odrębny dowód; agregator ma odrzucać brak, niezgodny SHA lub niepowodzenie.

**Nadal NO-GO:** bez docelowego systemd/cgroup service, niezależnych endpointów mTLS i dwóch tenantów, kontrolowanego SIGKILL/replay/network-partition, rotacji certyfikatów oraz niezależnego audytu i podpisanego odbioru operatora **nie oznaczać 0.38 jako ukończonego**. Nie zamawiać płatnego VPS bez osobnego zatwierdzenia kosztu.

## Aktualizacja dowodów: dwie rzeczywiste, efemeryczne maszyny Linux GitHub Actions (2026-10-14)

- Run [38610837380](https://github.com/mojealterego/ocaf-runtime/actions/runs/38610837380) dla SHA `df2522381965f430d623af6dcfe69230c45bed92`: **oba joby SUCCESS**.
- VM A: nieuprzywilejowana usługa `ocaf-twohost-coordinator.service` pod realnym systemd, cgroup limits, zatwierdzony digest Wasmtime 49.0.2, podpisany WASI; VM B: niezależny kernel boot ID, osobny proces klienta z certyfikatem mTLS. Potwierdzono cross-VM TLS-bound signed WASI RPC oraz odmowę nieautoryzowanemu agentowi B.
- Wyniki CI są syntetycznymi fixture'ami, nie produkcyjnym systemem dwóch tenantów. Nie przeprowadzono na tym runie długiej próby chaos dla partycji/sigkill i rotacji certyfikatów.
- Audyt negatywnego testu wykazał ryzyko fałszywie zielonego wyniku: dowolny niezerowy exit code klienta (np. zerwane połączenie) mógł zostać uznany za odmowę autoryzacji. Wzmocniono fail-closed: odrębne stdout/stderr, niezerowy exit, dokładna diagnoza odmowy z poziomu RPC; błędy sieci/timeout/certyfikatu wywołują NO-GO. Dodano testy regresyjne, a workflow uruchamia się również po zmianach `src/**`, `tests/**` i `Cargo.lock`.
- Zmiana niniejszego dokumentu uruchamia osobny test 5 × 20 Wasmtime CI soak na nowym SHA. Jego **wynik należy odczytać z GitHub Actions**, a nie zakładać sukcesu na podstawie samego triggera.

**Werdykt produkcyjny pozostaje NO-GO.** Pozytywne joby efemeryczne nie są zastępstwem trwałej infrastruktury staging, weryfikacji dwóch tenantów, chaosu/recovery, niezależnej oceny i podpisów operatora oraz audytora.

### Wynik ujemnego testu 38613269443 — RED potwierdzony

Poprzedni test rozróżniający transport od odmowy policyjnej zakończył się NO-GO, ponieważ agent B otrzymał podpisaną odpowiedź RPC o statusie FAILED (stdout), zamiast odmowy na poziomie ingress. W tym stanie nie można dowieść, że egzekucja została zatrzymana **przed** runtime. Przed kolejnym uruchomieniem jawnie ustawiono `allowed_tools=[]` dla agenta B w rzeczywistej konfiguracji koordynatora testowego; dodano dodatnią próbę uwierzytelnienia B przez mTLS i pusty discovery, po czym odmowa uruchomienia narzędzia musi nastąpić po stronie RPC. Taki test dowodzi polityki konkretnego agenta w CI; nadal nie dowodzi izolacji dwóch niezależnych tenantów na trwałym staging.

**Poprawka kontraktu polityki koordynatora:** `load_peers` celowo odrzuca `allowed_tools=[]`. Negatywny agent posiada wyłącznie niewrażliwy `demo_echo`, a NIE `ocaf_ci_noop`/tenant-bound WASI, co pozwala udowodnić uwierzytelnienie i policyjny zakaz wykonania narzędzia bez osłabiania fail-closed parsera.

## Rzeczywiste wyniki CI dostępne w GitHub Actions (2026-10-14)

1. [run 38614920507](https://github.com/mojealterego/ocaf-runtime/actions/runs/38614920507) — dwa różne Linux kernel boot ID, TLS-bound mTLS pomiędzy maszynami, podpisany WASIp1 uruchomiony w systemd na VM A i odmowa signed-WASI agentowi B. SHA `e442734f27e49e640941b77eee7f11332cd180f3`, joby PASS.
2. [run 38617180984](https://github.com/mojealterego/ocaf-runtime/actions/runs/38617180984) — dwa współbieżne tenanty `tenant-a` i `tenant-b` na jednym prawdziwym efemerycznym hoście Linux, oddzielne CA Ed25519/PKI, unprivileged systemd cgroups, przypięty Wasmtime 49.0.2; 7/7 zamkniętych prób: autoryzowana egzekucja obu, 12 równoległych RPC, cross-principal odmowa przed engine, krzyżowe TLS CA, przerwanie transportu A przy dalszej pracy B, SIGKILL/restart A z nowym PID i zachowaniem B, żywa zamiana/revocation TLS client CA i przywrócenie przy pracy B. SHA `19c639eb10ca42f778eb9f58ef55a347757c59b2`, job PASS.
3. W obu przypadkach wykonano kod i pomiary rzeczywistych procesów w CI, ale fixture są syntetyczne, węzły krótkotrwałe i nie są to trwałe systemy docelowe. Lokalny transport fault na jednym runnerze nie jest rzeczywistą partycją pomiędzy niezależnymi hostami. **NO-GO** pozostaje wiążące, dopóki audytor i operator nie zatwierdzą spójnego pakietu na finalnym SHA.

Zakres przekazania do niezależnej weryfikacji: [STAGE038_INDEPENDENT_AUDIT_HANDOFF.md](STAGE038_INDEPENDENT_AUDIT_HANDOFF.md).
