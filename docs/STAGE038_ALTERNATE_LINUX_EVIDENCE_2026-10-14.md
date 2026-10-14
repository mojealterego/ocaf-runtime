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
