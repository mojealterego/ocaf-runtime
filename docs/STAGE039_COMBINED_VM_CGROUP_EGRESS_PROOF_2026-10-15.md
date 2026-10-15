# OCAF 0.39 — rzeczywisty jailed Firecracker + Aya w hostowym cgroup VM

**Data obserwacji:** 2026-10-15. **Repo:** `mojealterego/ocaf-runtime`,
gałąź `main`. **Zakres:** efemeryczny, uprzywilejowany GitHub-hosted
Ubuntu 24.04. Żadna nowa maszyna płatna nie została wdrożona.

## Materiał źródłowy

- [GitHub Actions 38711385989](https://github.com/mojealterego/ocaf-runtime/actions/runs/38711385989)
  – `live-jailer-kvm-guest`, `conclusion=success`.
- Testowany kod `main`: `2363d6e193710bf499d0c33b505ffd88a8fa5d4d`.
- [Artefakt 11960946253](https://github.com/mojealterego/ocaf-runtime/actions/runs/38711385989/artifacts/11960946253)
  `stage039-real-firecracker-jailer-linux-evidence`, upload `success`.
- SHA-256 przesłanego ZIP raportowane przez Actions:
  `7873fbb4affc0bb33677c6cd3a2f3e7121d11ca2a6f1a2148487e95c5e0ab69b`.
  Raporty: `kvm.json`, `guest-boot.json`, manifesty digestów.

## Zarejestrowane wyniki

| Obserwacja na prawdziwym kernel Linux | Wynik |
| --- | --- |
| KVM ioctl `KVM_CREATE_VM` | PASS; `STAGE039_REAL_KVM_KERNEL_CREATE_VM_PROVED` |
| Pinned Firecracker 1.17.0, jailer, guest kernel/rootfs | PASS na wejściowych SHA-256 |
| Guest Linux boot, API `Running`, członkostwo PID w cgroup v2 | PASS; `JAILED_FIRECRACKER_GUEST_KERNEL_BOOT_OBSERVED` |
| Aya BPF attach do tego samego hostowego cgroup żywej VM | PASS, weryfikacja kernel program ID i inode |
| Hostowy próbnik A ma TCP do loopback przed attach | PASS |
| Hostowy próbnik A traci TCP po attach deny-all | PASS |
| Hostowy próbnik w odrębnym cgroup B zachowuje TCP | PASS |
| Hostowy próbnik A odzyskuje TCP po detach Aya | PASS |
| VM nadal żywa w czasie dowodu, Aya odpięta | PASS |
| Twierdzenie o produkcyjnym GO | `production_approved=false` |
| Ruch wychodzący z systemu guest VM | **NIEBADANY** |

GitHub Actions wypisał kolejno:
`STAGE039_REAL_FIRECRACKER_JAILER_GUEST_LINUX_BOOT_PROVED` oraz
`STAGE039_LIVE_VM_HOST_CGROUP_AYA_EGRESS_NEGATIVE_AND_RECOVERY_PROVED`.

## Interpretacja i otwarte warunki

Hostowy proces potomny próbkujący TCP został umieszczony w grupie cgroup
procesu Firecracker. Program Aya `cgroup_skb/egress` egzekwował
lokalne deny-all **na tym cgroup**. Gość uruchomiony przez Firecracker
w tej próbie otrzymał `network-interfaces=[]`; nie istnieje
wiarygodna próbka pochodzącego z gościa TCP/UDP, więc **nie wolno**
traktować tego wyniku jako dowodu guest-vs-host sieci, wielodzierżawności,
braku VM escape czy fizycznie niezależnych dwóch hostów.

Realne kolejno wymagane próby: sieć gościa (jawnie kontrolowany
interfejs TAP i polityka), izolacja dwóch równoczesnych gości,
zachowanie przy przerwaniu attach/restartach/partycjach i pomiary
długiego czasu na trwałym staging; rotacja tożsamości i niezależny audyt.

**Status:** `stage_038_project_status=COMPLETE` na polecenie operatora
(zamknięcie administracyjne), `stage_038_security_acceptance=INCOMPLETE`,
`stage_039=IN_PROGRESS`, `production_approved=false`.
