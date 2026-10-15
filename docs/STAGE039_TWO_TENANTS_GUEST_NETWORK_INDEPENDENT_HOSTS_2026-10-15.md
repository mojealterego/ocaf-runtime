# OCAF 0.39 — rzeczywiste dwa tenanty, guest UDP i dwa niezależne kernele Linux

**Data dowodów:** 2026-10-15. **Gałąź:** `main`.

## Trzy wykonane próby sprzętowe

| ID | Próbka rzeczywista | Wynik na obserwowanym commicie | Granica wyniku |
| --- | --- | --- | --- |
| LAB-2VM | Dwa równocześnie uruchomione jailed Firecracker/KVM z różnymi `tenant_id`, `agent_id`, `workload_id`, UID/GID, VM ID, inode cgroup; SIGKILL VM A podczas pracy VM B | **PASS** — [workflow 38715926542](https://github.com/mojealterego/ocaf-runtime/actions/runs/38715926542) | Jeden efemeryczny runner GitHub Actions |
| LAB-GUEST-NET | Dwa prawdziwe goście Firecracker inicjują UDP ze statycznego procesu PID1 do osobnych TAP; `nftables` drop wyłącznie gościa A, B działa, A odzyskuje po usunięciu reguły, B działa po SIGKILL VM A | **PASS** — ten sam [workflow 38715926542](https://github.com/mojealterego/ocaf-runtime/actions/runs/38715926542) | Osobny lab Firecracker **bez jailer**, izolacja ruchu przez host `nftables`, nie Aya guest-egress |
| LAB-2HOST | Dwa niezależne joby `ubuntu-24.04`, każdy własny `KVM_CREATE_VM`, jailed Firecracker/guest Linux, własne `/proc/sys/kernel/random/boot_id`, własny SIGKILL i `cgroup.events populated=0`; niezależny trzeci job porównuje dowody | **PASS** — [workflow 38720667372](https://github.com/mojealterego/ocaf-runtime/actions/runs/38720667372) | Dwa efemeryczne kernele, **nie** dwa trwałe production hosty ani inter-host failover |

## Weryfikowalne artefakty i identyfikatory

**Dwa VM i pakiety wychodzące z guest:** [artifact 11962478297](https://github.com/mojealterego/ocaf-runtime/actions/runs/38715926542/artifacts/11962478297).
Archiwum ZIP SHA-256 z logu `actions/upload-artifact`:
`de80a6deeb88fdb8e9e7f9ef1493ba2c2bdc33b80a29e04ddbccffe7c846a801`.
Log zawiera wyniki:
- `STAGE039_REAL_TWO_SIMULTANEOUS_JAILED_FIRECRACKER_TENANTS_PROVED`.
- `STAGE039_TENANT_B_SURVIVED_TENANT_A_SIGKILL_PROVED`.
- `STAGE039_REAL_GUEST_ORIGINATED_NETWORK_TENANT_DENY_RECOVERY_PROVED`.

**Niezależny kernel / runner tenant A:** `boot_id` =
`39a9d1b0-757b-48b6-a255-f157ffe3e5ec`,
[artifact 11964199623](https://github.com/mojealterego/ocaf-runtime/actions/runs/38720667372/artifacts/11964199623),
ZIP SHA-256 `75baec920cb65daf437b3a0b9e3970ee68a8428cdab2be70650ecf373a5c8255`.

**Niezależny kernel / runner tenant B:** `boot_id` =
`e20b7a42-963a-4a1f-893b-65a807a5091d`,
[artifact 11965540631](https://github.com/mojealterego/ocaf-runtime/actions/runs/38720667372/artifacts/11965540631),
ZIP SHA-256 `b22217a7cbf10142788b6f8cb554d2c099ee7d2369d1f88e1627a9f62cf6c86e`.

**Końcowe porównanie hostów:** [artifact 11964930644](https://github.com/mojealterego/ocaf-runtime/actions/runs/38720667372/artifacts/11964930644),
ZIP SHA-256 `440e1e6a0441560b93af6c6ec5319b31d941d447f41a171c67ac3f4ba2feb419`,
status `TWO_INDEPENDENT_EPHEMERAL_KERNEL_HOST_KVM_SIGKILL_PROVED`,
`production_approved=false`. Każdy job i końcowa walidacja mają
`conclusion=success`. Ich własne `boot_id` są różne.

**Rzeczywisty błąd i naprawa:**
Pierwsze dwa joby hostowe zawiodły z powodu długości ścieżki do
`firecracker.socket` (**118 bajtów**, >107 bajtów dopuszczalnych przez
`sockaddr_un.sun_path` dla ścieżki zakończonej NUL).
Skrócono jailer root do `/opt/o39h<run>-<a|b>`, dodano
`validate_api_socket_path` oraz test regresyjny. Kolejne dwa joby
uruchomiły VM i wykonały SIGKILL. W pierwszej końcowej agregacji obydwa
joby A/B miały PASS, ale niepotrzebny warunek różnych `hostname`
zwrócił FAIL; obraz chmurowego runnera może używać powtarzalnej nazwy.
Wymuszenie dwóch różnych kernel `boot_id` pozostawiono,
a hostname udokumentowano jako informację pomocniczą. Cały późniejszy
run 38720667372 to rzeczywisty **SUCCESS**.

## Rzeczywiste granice i nadal niezrealizowane próby

- Dwa goście z **jailer** zostały przetestowane równocześnie, ale bez
  skonfigurowanej sieci gościa; osobny lab **guest-initiated UDP** był
  uruchomiony na dwóch gościach Firecracker **bez jailer**.
  Nie wolno łączyć tych dwóch wyników w nieistniejący jednoczesny
  dowód `guest UDP + jailer + Aya eBPF`.
- Regułę deny/recovery pakietów gościa wymusił `nftables` na oddzielnych
  interfejsach TAP; **nie** dowodzi to, że Aya `cgroup_skb/egress`
  przyjmuje lub blokuje pakiety wychodzące z sieci guest.
- Dwa różne `boot_id` i osobne joby potwierdzają dwa niezależne uruchomienia
  systemów Linux/KVM, lecz nie potwierdzają fizycznie niezależnych,
  długotrwałych hostów produkcyjnych ani izolacji od awarii całego
  centrum danych.
- Oba VM SIGKILL nastąpiły na odrębnych efemerycznych runnerach;
  **nie wykonano** wyłączenia fizycznego hosta, partycji połączenia
  między tymi hostami, prawdziwego dwuhostowego mTLS RPC/failover,
  migracji stanu lub odtworzenia po awarii hosta.
- Nie ma niezależnego podpisanego audytu ani produkcyjnego odbioru
  tenantów na docelowej infrastrukturze.

**Status:** `0.38 COMPLETE / CLOSED_BY_OPERATOR` (projektowo),
`0.39 IN_PROGRESS`, `production_approved=false`,
`stage_039_complete=false`. Wszelkie niepotwierdzone przypadki
pozostają w bramce produkcyjnej `NO-GO`.
