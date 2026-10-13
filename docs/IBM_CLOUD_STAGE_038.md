# IBM Cloud VPC — staging OCAF, odbiór etapu 0.38

**Środowisko:** IBM Cloud Virtual Server for VPC (VSI), Linux x86_64,
Ubuntu lub inna zgodna dystrybucja z systemd i cgroup v2.
**Cel:** zebrać dowody odbioru bezpieczeństwa OCAF WASIp1 **bez**
naruszania środowiska produkcyjnego.
**Status:** projekt procedury. Nie uruchomiono żadnej instancji i nie
nadano żadnych uprawnień IBM Cloud z poziomu repozytorium.

## Istniejący kontekst IBM Cloud

W osobnym prywatnym repozytorium `mojealterego/IBM-Cloud-automation`
istnieje konfiguracja Code Engine/IAM dla regionu `eu-de`,
grupy zasobów `Project-X`, projektu
`ugm-agentic-control-plane` i profilu zaufanego
`ugm-control-plane-runtime`. Dokument
`docs/IMPLEMENTATION_STATUS.md` tego repozytorium opisuje
region, grupę zasobów i profil jako wcześniej potwierdzone
przez właściciela, lecz nie potwierdza żadnej **działającej VSI**
ani wdrożenia OCAF.

**Ważne:** IBM Code Engine dla kontrolera agentów
to inny typ uruchomienia niż pełny host Linux/systemd.
Testy cgroup v2, procesów potomnych, `SIGKILL` i
hostowego OOM wymagają własnej maszyny
**IBM Cloud Virtual Server for VPC (VSI)**. Nie uznawać
istniejącego projektu Code Engine za maszynę testową.
Nowy VSI powinien korzystać z istniejącej zatwierdzonej
grupy `Project-X`, jeśli operator tak zdecyduje;
nie wolno tworzyć jej drugi raz przez Terraform
bez sprawdzenia istniejącego stanu.

Dla pierwszej inspekcji w IBM Cloud kieruj polecenia
do regionu `eu-de` (region pozostaje konfigurowalny):
`ibmcloud target -r eu-de`, następnie
`ibmcloud is instances`. Te polecenia nie tworzą zasobów.

## 1. Architektura

1. **VPC staging** — istniejący lub wydzielony IBM Cloud VPC, prywatna
   podsieć, osobna grupa zasobów, reguły security group *default deny*
   na ruch przychodzący. Nie wystawiać portów OCAF RPC publicznie.
2. **Host A i host B (preferowane)** — dwie instancje VSI Linux x86_64
   w jednej strefie lub wielu strefach regionu. Host A koordynator
   i signed-WASI executor, host B izolowany worker; dwa syntetyczne
   tenanty, certyfikaty testowe mTLS. Jedna VSI może wykonać preflight
   i lokalny stress smoke, ale **nie** wielohostowy test partition.
3. **Dostęp administracyjny** — IBM Cloud VPN for VPC albo oddzielny
   bastion z SSH ograniczonym do kontrolowanych adresów. Nie otwierać
   SSH 22/tcp dla `0.0.0.0/0`; nie instalować publicznego endpointu
   dla usługi OCAF bez odrębnego projektu ochrony.
4. **Systemd+cgroup** — zatwierdzić szablon
   [jednostki OCAF](../deploy/systemd/ocaf-coordinator-wasi.service.example).
   Dla nowych staging VSI wstępnie planować >= 4 vCPU / 16 GiB RAM
   przy kompilacji Rust i testach; koszty i rzeczywiste wymagania
   potwierdzić przez kalkulator IBM Cloud i pomiary. To nie jest
   wiążący dobór profilu.
5. **Artefakty dowodowe** — raporty nie zawierające sekretów przechowywać
   w zatwierdzonym repozytorium evidence/IBM Cloud Object Storage,
   z ograniczonym IAM i określoną retencją. Nie publikować kluczy KMS,
   tokenów IAM ani prywatnych PEM w GitHub Actions.

## 2. Odczyt konta IBM Cloud (bez tworzenia zasobów)

Na **zaufanej stacji operatora**, po autoryzowanym logowaniu przez SSO:

```bash
ibmcloud login -a https://cloud.ibm.com --sso
ibmcloud target
ibmcloud is regions
ibmcloud is instances
# Po wybraniu konkretnej instancji, bez wprowadzania tu prywatnych danych:
ibmcloud is instance INSTANCE_NAME_OR_ID --output JSON
```

Instancje w regionach innych niż aktualny wymagają właściwego
`ibmcloud target -r REGION`. IBM VPC jest usługą regionalną.
**Nie uruchamiać** `instance-create`, `instance-delete` ani Terraform
`apply` bez zatwierdzenia kosztów, planu sieci i zakresu IAM.

W konsoli IBM Cloud: **Infrastructure → VPC → Compute → Virtual server
instances**. Potrzebne są co najmniej: region, zone, typ usługi
(VPC VSI / Classic / Bare Metal / ROKS), stan istniejącej maszyny,
system operacyjny i uzgodniony sposób SSH/VPN. Nie przekazywać
prywatnego klucza SSH ani API key w rozmowie.

## 3. Preflight bez modyfikacji hosta

Po uzyskaniu autoryzowanego dostępu SSH/VPN do **wydzielonej VSI**:

```bash
git clone https://github.com/mojealterego/ocaf-runtime.git
cd ocaf-runtime
git rev-parse HEAD
python3 scripts/ibm_cloud_wasi_preflight.py \
  --report ./ibm-vpc-038-preflight.json
```

Preflight niczego nie instaluje ani nie uruchamia jako usługi.
Sprawdza: Linux x86_64, `/proc/self/fd`, PID 1 = systemd,
cgroup v2 CPU/memory/pids, możliwość uszczelnienia `memfd`,
`PR_GET_PDEATHSIG`, `cargo`, `git`. Raport jest tworzony tylko,
gdy podana ścieżka jeszcze nie istnieje; prawa tworzenia 0600.
Brak RAM >= 8 GiB jest ostrzeżeniem, nie automatycznie GO.

## 4. Odbiór wstępny i testy rzeczywiste

Po przeglądzie policy i zatwierdzeniu operatora:

```bash
cargo test --locked --no-default-features --test wasi_sandbox -- --test-threads=1
bash scripts/wasmtime_acceptance.sh

# Pełne 100 cykli na approved real Wasmtime z hash przypiętym w
# zaufanym konfiguratorze; limity trzeba dopasować do profilu VSI.
OCAF_TEST_WASMTIME=/opt/ocaf/wasmtime/wasmtime \
OCAF_TEST_WASMTIME_SHA256='<operator-approved-sha256>' \
python3 scripts/wasi_host_acceptance.py \
  --cycles 100 --timeout 120 --max-fds 512 --max-processes 128 \
  --report ./ibm-vpc-038-soak.json
```

Kolejnym krokiem po preflight jest **oddzielne zatwierdzenie**
konfiguracji systemd, metryk cgroup `memory.peak`/`pids.peak`,
chaos SIGKILL, awarii sieci i testu między dwoma hostami.
Szczegóły i macierz podpisów:
[STAGE_038_OPERATOR_SIGNOFF.md](STAGE_038_OPERATOR_SIGNOFF.md).

## 5. Minimum IAM i zasady bezpieczeństwa

- Inspekcja może używać ról o uprawnieniach **read-only**.
  Wdrażanie VSI, przypinanie security group czy tworzenie DNS wymaga
  dodatkowego upoważnienia operatora konta.
- Oddzielić operatora infrastruktury od procesu wykonującego WASM.
  Podpisywać manifesty offline/KMS; na executorze wyłącznie publiczny
  klucz publikatora z owner-only odczytem.
- Nie udostępniać tokenów IBM IAM, certyfikatów prywatnych i danych
  konta w repozytorium lub logach CI. Używać zaufanego menedżera
  sekretów i rotacji.
- Przed testami SIGKILL/OOM upewnić się, że VSI jest **staging-only**,
  nie obsługuje prawdziwych żądań i nie posiada aktywnych efektów
  zewnętrznych.

## 6. Kryterium decyzji

**CI SUCCESS = testy kodu. IBM Cloud staging GO** wymaga również
gotowego bezpiecznego hosta, izolacji tenantów, pomiarów pod obciążeniem,
negatywnych testów awaryjnych, rotacji certyfikatów i kluczy,
raportu niezależnego audytu oraz podpisanego odbioru operatora.
Do tego czasu **0.38 PARTIAL / PRODUCTION NO-GO**, etap 0.39 zablokowany.

### Oficjalne źródła IBM Cloud

- https://cloud.ibm.com/docs/vpc?topic=vpc-creating-virtual-servers
- https://cloud.ibm.com/docs/vpc?topic=vpc-using-security-groups
- https://cloud.ibm.com/docs/vpc?topic=vpc-vpc-secure-management-bastion-server
- https://cloud.ibm.com/docs/vpc?topic=vpc-vpc-reference
- https://cloud.ibm.com/docs/vpc?topic=vpc-vpn-client-to-site-security-groups

## 7. Bramka kosztowa i architektura kolejnych etapów

Budżet IBM Cloud udostępniony przez właściciela nie jest automatycznie
budżetem możliwym do wydania na każdy produkt. Przed zamówieniem czegokolwiek
operator musi sprawdzić w konsoli **Manage → Billing and usage →
Promotions and credits** datę wygaśnięcia, typ kredytu, kwalifikujące
się usługi i ewentualny limit miesięczny:

- https://cloud.ibm.com/docs/account?topic=account-applying-promo-codes
- https://cloud.ibm.com/docs/account?topic=account-billusagefaqs

Proponowana **początkowa bramka zatwierdzenia** tylko dla staging 0.38:
`max_estimated_pilot_cost_usd = 500`. To limit planowania i ręcznej
akceptacji kosztorysu, **nie rzeczywisty limit rozliczeniowy**.
Środowisko i dane operatora są prywatne; do repozytorium nie
wprowadzamy salda konta. Wycenę należy uzyskać dla profili i regionu
`eu-de` w IBM Cloud Cost Estimator, z uwzględnieniem VSI, boot
volumes, egress, floating IP, log storage i innych usług:

- https://www.ibm.com/products/cloud/cloud-calculator

Rozpocznij od jednej VSI `bx2-4x16` albo `bx4-4x16` (4 vCPU,
16 GiB, jeśli profil oferowany w `eu-de`) na read-only preflight
oraz lokalny 0.38 soak. Druga VSI jest potrzebna dopiero do
wielo-węzłowej izolacji i partition/chaos. **Nie twórz ani
nie uruchamiaj maszyn** bez audytu istniejących zasobów, kosztorysu
i wyraźnej zgody na zasoby płatne.

IBM Cloud spending notifications mogą wysyłać alerty przy 80%, 90%
i 100% zadeklarowanego progu, ale **nie blokują naliczania opłat**:

- https://cloud.ibm.com/docs/support?topic=support-spending
- https://cloud.ibm.com/docs/account?topic=account-billusagefaqs

**Pułapka architektury 0.39:** IBM Cloud oficjalnie **nie obsługuje
nested virtualization na instancjach VSI**, więc testów
Firecracker/KVM (`/dev/kvm`) nie wolno planować na tym samym rodzaju
serwera, co testy WASIp1 0.38. Potrzebny będzie osobno dobrany
**IBM Cloud Bare Metal Server for VPC** z dostępem do KVM,
alternatywny zatwierdzony host fizyczny albo wariant bez microVM.
Przed kosztem Bare Metal należy potwierdzić rzeczywistą dostępność
`/dev/kvm` oraz cenę, licencje i warunki kredytów.
Wyłączenie Bare Metal może **nie zatrzymać naliczania opłat**;
właściwe zakończenie wymaga deprowizjonowania po odbiorze.

- https://cloud.ibm.com/docs/vpc?topic=vpc-troubleshoot-enable-nested-virtualization
- https://cloud.ibm.com/docs/vpc?topic=vpc-about-bare-metal-servers
- https://cloud.ibm.com/docs/vpc?topic=vpc-bare-metal-server-faq

Wszystkie płatne operacje mają pozostać ręcznie zatwierdzane.
Brak dostępu do konta IBM Cloud oznacza brak faktycznego wdrożenia.
