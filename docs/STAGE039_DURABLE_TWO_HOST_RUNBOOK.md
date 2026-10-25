# 0.39 — dwa trwałe hosty KVM: uruchomienie przez operatora

Ten runbook przygotowuje dowody z dwóch **własnych, trwałych** hostów Linux/KVM
`A` i `B`. GitHub-hosted CI nie jest takim środowiskiem. Operator przechowuje
raporty poza repozytorium i sam sprawdza tożsamość maszyn. Skrypty nie składają
podpisu właściciela ani nie ustawiają `stage_039_complete=true`.

## Warunki wejściowe

- Dwa różne hosty z `/dev/kvm`, cgroup v2, `sudo` i trwałym dyskiem; ich
  `machine-id` oraz DMI `product_uuid` muszą pozostać stałe po restarcie.
- Ten sam przypięty commit repozytorium na obu maszynach. Na hostach trzeba
  zainstalować Python 3, `iproute2`, `nftables`, `openssl`, Firecracker 1.17,
  jailer 1.17 i obrazy testowe, zgodnie z hashami i poleceniami w
  `.github/workflows/stage039-independent-hosts.yml`.
- Na każdym hoście przygotować oddzielny, prywatny katalog
  `/opt/o39h<IDENTYFIKATOR>-a` lub `...-b`, z `approved/` i `jailer/`
  należącymi do `root:root` i trybem `0700`. W `approved/` umieścić
  `firecracker`, `jailer`, `guest_kernel`, `guest_rootfs`,
  `aya_cgroup_device` oraz `request.json` o formacie dokładnie takim jak
  w kroku „Construct separately provisioned root-owned tenant image registry”
  wspomnianego workflow. Podane tam SHA-256 dla kernel/rootfs/BPF są bramką.
  Wartości `tenant_id` i `agent_id` muszą wynosić odpowiednio
  `tenant-a`/`agent-a` albo `tenant-b`/`agent-b`.

Przed pierwszym uruchomieniem operator potwierdza, że to naprawdę dwa
utrzymywane hosty, zapisuje ich pochodzenie i sprawdza trasę hosta A do
adresu IPv4 B. DMI i `machine-id` są wskaźnikami stabilności, nie atestacją
sprzętową. Nie przesyłać kluczy prywatnych do repozytorium.

## Sto cykli przed i po zimnym restarcie

Na A (dla B zamienić `a` na `b`, `tenant-a` na `tenant-b`, UID `1002` na
`1003`; używać oddzielnego `PREFIX` na B):

```bash
PREFIX=/opt/o39h20261025-a  # zastąp rzeczywistą ścieżką rejestru A
mkdir -m 0700 -p "$HOME/ocaf039-before"
sudo -n python3 scripts/stage039_kvm_soak.py \
  --prefix "$PREFIX" --tenant tenant-a --uid 1002 \
  --report "$HOME/ocaf039-before/soak.json" \
  --ack-real-kvm-100-cycle-soak
sudo -n chown "$(id -u):$(id -g)" "$HOME/ocaf039-before/soak.json"
python3 scripts/stage039_durable_host_bundle.py capture \
  --tenant tenant-a --soak "$HOME/ocaf039-before/soak.json" \
  --report "$HOME/ocaf039-before/host.json"
```

Zebrać oba raporty `before` do prywatnego katalogu kontrolera. Wykonać
**zimny restart obu maszyn** pod kontrolą operatora i potwierdzić nowe
`/proc/sys/kernel/random/boot_id`. Potem powtórzyć identyczne polecenia,
zmieniając tylko katalog `ocaf039-before` na `ocaf039-after`. Na każdym
hoście będzie wtedy 200 realnych cykli, razem 400. Porównanie na kontrolerze:

```bash
python3 scripts/stage039_durable_host_bundle.py compare \
  --a-before /PRIVATE/A-before/host.json \
  --b-before /PRIVATE/B-before/host.json \
  --a-after /PRIVATE/A-after/host.json \
  --b-after /PRIVATE/B-after/host.json \
  --report /PRIVATE/durable-pair.json
```

Skrypt odrzuca klonowaną tożsamość, brak zmiany boot ID, brak któregokolwiek
cyklu, powtórzony VM ID i zmianę łańcucha SHA-256. Raport nadal zawiera
`host_identity_cryptographically_attested=false`.

## Rzeczywista partycja sieci host A → host B

Na B uruchomić istniejący `scripts/stage039_mtls_fencing.py serve` z
prywatną bazą SQLite oraz CA, certyfikatem i kluczem serwera. Certyfikat
serwera musi mieć DNS SAN `ocaf039-broker.local`; certyfikat klienta A
DNS SAN `ocaf039-tenant-a.client`. CA i klucze należy wydać i pinować w
kanale operatora, osobno od efemerycznej CA z labu. Serwer nasłuchuje na
rzeczywistym adresie IPv4 B osiągalnym z A:

```bash
HOST_B_IPV4='ADRES_IPV4_HOSTA_B'
python3 scripts/stage039_mtls_fencing.py serve \
  --host "$HOST_B_IPV4" --port 24399 \
  --cert /PRIVATE/server.pem --key /PRIVATE/server.key \
  --ca /PRIVATE/ca.pem --db /PRIVATE/ledger/state.sqlite \
  --ready /PRIVATE/server.ready
```

`/PRIVATE/ledger` musi być prywatnym katalogiem `0700`. Na A, po
potwierdzeniu trasy i gotowości serwera, wykonać:

```bash
HOST_B_IPV4='ADRES_IPV4_HOSTA_B'
sudo -n python3 scripts/stage039_interhost_partition.py \
  --host-b-ipv4 "$HOST_B_IPV4" --port 24399 \
  --cert /PRIVATE/client.pem --key /PRIVATE/client.key \
  --ca /PRIVATE/ca.pem --report /PRIVATE/interhost-partition.json \
  --ack-real-host-network-partition
```

Skrypt najpierw otrzymuje grant TLS 1.3, następnie w jądrze A instaluje
regułę `nftables` odcinającą tylko TCP do `<ADRES_B>:24399`. Renewal musi
zawieść **i licznik pakietów tej dokładnej reguły musi wzrosnąć**; sam timeout
nie jest dowodem partycji. Raport zawiera liczbę pakietów przed i po próbie.
Przywrócenie trasy musi pozwolić na renewal i release. Reguła jest
usuwana także po błędzie. Nie dotyka portu SSH. Na maszynie produkcyjnej
operator nadal odpowiada za kontrolę pozostałych reguł i łączności.

To **dowód partycji transportu**, nie test promocji drugiego brokera ani
kworum. `multi_host_failover_tested=false` pozostaje w raporcie. Do pełnej
bramki 0.39 potrzebny jest osobny test utraty hosta B, przejęcia przez
fenced authority, niedopuszczenia dwóch primary i odrzucenia starego epoch.

## Mapa do dossier właściciela

| Plik dowodowy v2 | Zawartość po uruchomieniu |
| --- | --- |
| `independent_kernel_kvm_hosts` | 4 raporty `host.json`, 4 raporty `soak.json`, `durable-pair.json`, źródłowy commit i identyfikacja operatora. |
| `two_kernel_cold_restart` | Zapis wykonanych restartów, stałe DMI/machine ID i nowe boot ID z `durable-pair.json`. |
| `kernel_tls13_mtls_partition_fencing` | `interhost-partition.json`, przypięte certyfikaty, log serwera B i zrzut stanu sieci. |
| `durable_host_partition_soak` | Połączenie obu zestawów 100 cykli i testu partycji z ręcznym odczytem operatora. |
| `two_jailed_tenant_guest_kernel_ebpf` | Osobny dowód z aktualnego workflow KVM z Aya/WASI; sam prosty soak go nie zastępuje. |
| `production_interhost_mtls_ha` | Wymaga dodatkowej rzeczywistej próby failover/dual-primary; ten runbook jej nie dostarcza. |
| `operator_formal_approval` | Decyzja i podpis właściciela według `docs/STAGE039_OWNER_SIGNOFF.md`. |

Nie tworzyć pustych ani syntetycznych plików dla brakujących dowodów.
