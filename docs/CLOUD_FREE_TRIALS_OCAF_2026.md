# OCAF — audyt darmowych i promocyjnych hostów do odbioru 0.38 (2026-10-14)

**Status: VERIFY BEFORE USE.** Źródła: oficjalne strony usługodawców
sprawdzone 14 października 2026. Oferty mogą zmienić się bez
powiadomienia; żadne konto, klucz, maszyna ani środki nie były
aktywowane lub wydane w trakcie sporządzania tej oceny.
Kod projektu pozostaje na gałęzi **`main`**.

## Co dokładnie musi zapewniać serwer OCAF 0.38?

OCAF na etapie 0.38.8–0.38.9 nie potrzebuje jedynie „VPS z rootem”.
Do faktycznej walidacji zatwierdzonego wykonawcy WASIp1 wymagane są:
Linux x86_64; kernel `memfd_create` + `F_ADD_SEALS` + `prctl`;
`/proc/self/fd`; systemd jako PID 1; cgroup v2 z
`cpu memory pids`; kontrola zakresu `SIGKILL`; stabilny
dostęp administracyjny do testu wielogodzinnego i możliwość
sprawdzenia izolacji procesów na **dwóch hostach**.
Weryfikacja produkcyjna wymaga również rozdzielonych tenantów,
transportu TLS/mTLS, izolacji sieci i podpisanego odbioru.
`KVM VPS` jako **rodzaj hosta** nie gwarantuje, że jego gość
może uruchomić własne microVM: do etapu 0.39 osobno wymagane są
`/dev/kvm`, sprzętowa nested virtualization oraz ocena ryzyka
migracji hosta.

Obecny `scripts/wasmtime_acceptance.sh` ma przypięty upstream
Wasmtime **49.0.2 Linux x86_64** i wprost kończy się błędem
na `aarch64`. Testowanie ARM jest wartościowe, ale nie może
zostać przedstawione jako zaliczony x86_64 gate bez niezależnie
przypiętego, przebadanego binarium i adaptacji testów.

## Porównanie usług i ich zastosowanie w OCAF

| Usługa | Potwierdzona oferta / ważne ograniczenie | Przydatność dla OCAF |
| --- | --- | --- |
| VDS.GURU | Strona główna pokazuje „24 Hours Free”, ale **w innym miejscu również 7 dni trial**. Dostęp do szczegółowej ścieżki `/trial/` nie powiódł się (HTTP 502); nie potwierdzono braku wymogu płatności ani precyzyjnego terminu. Płatne VM opisane jako KVM VPS z root. | **Warunkowo P2**: krótki Linux preflight oraz deterministyczne testy sieci po weryfikacji warunków i uprawnień; nie planować wielodniowego soak. |
| Cloudways Flexible/Autonomous | 3 dni bez karty według oficjalnego help center. **Cloudways nie daje root** i zachowuje kontrolę nad bazowym serwerem. | **Odrzucić dla host acceptance 0.38**: dopuszczalny jedynie test aplikacyjnego HTTP frontend, nie `systemd`, host cgroup i OOM. |
| h2.nexus | 15 minut, przeglądarkowy root Debian 11, 4 vCPU / 8 GB RAM / 60 GB SSD; po czasie zasoby i dane znikają. | **P3 smoke/edukacja**: jednorazowa inspekcja `uname`, `/proc`, cgroups; za krótko na kompilację OCAF, 100-cyklowy soak i dwuhostowe testy odbiorowe. Nie podawać sekretów. |
| Oracle Cloud Always Free | Oficjalne, szczegółowe aktualne docs określają Ampere A1 **1 500 OCPU-h i 9 000 GB-h/mies.**, co daje około **2 OCPU / 12 GB RAM** w modelu Always Free. Dwie mikro-VM AMD mają po 1 GB RAM. Strony marketingowe nadal opisują 4 OCPU/24 GB; sprzeczne informacje rozstrzyga panel konkretnego tenanta. Wymaga karty/weryfikacji; A1 ARM64, możliwy brak wolnych maszyn i odzyskiwanie nieaktywnych instancji. | **P1 długoterminowa sieć/control plane** na ARM; na AMD micro tylko mały coordinator/monitoring. Obecny prawdziwy pin Wasmtime 49.0.2 wymaga x86_64. |
| Google Cloud Always Free | 1 `e2-micro` tylko w trzech regionach USA (`us-west1`, `us-central1`, `us-east1`) w ramach limitów dysku i egress; potrzebne konto rozliczeniowe. | **P2 malutki worker/monitoring**, nie pełne testy OOM/soak ani dwie duże instancje. |
| Kamatera | **30 dni, jedna VM do 100 USD** i 1 TB ruchu, ważna karta; przy nadwyżce lub po zakończeniu mogą wystąpić opłaty. | **P1 niezależny Linux x86_64** do krótkiego soak, opcjonalnie jako drugi dostawca; zakres tylko jednego serwera i wymagana kontrola kosztów. |
| OVHcloud Public Cloud | Pierwszy kwalifikowany projekt: 1000 PLN/30 dni; metoda płatności obowiązkowa; po wyczerpaniu kredytów mogą powstać koszty. **Nie dotyczy standardowego VPS poza Public Cloud.** | **P1 krytyczny staging** na 1–2 Linux VSI x86_64, systemd/cgroups, sieć prywatna. Zobacz dedykowany plan. |
| Mikrus FROG | 256 MB RAM / 3 GB dysku, Alpine Linux, **LXC**, jednorazowa wpłata weryfikacyjna minimum 5 PLN; 1 darmowa maszyna. | **Wyłącznie mały monitoring/heartbeat**. Nie nadaje się do pełnego Rust build i nie zapewnia oczekiwanego hostowego systemd/cgroup. |

**P1/P2/P3 oznacza priorytet badawczy, nie status konta, nie
dostępność i nie automatyczną zgodę na aktywację.**

### Źródła pierwotne (w tym zidentyfikowane sprzeczności)

- VDS.GURU: https://vds.guru/ — „24 Hours Free” i „7 days risk-free trial” na tej samej stronie; https://vds.guru/trial/ niedostępna przy weryfikacji.
- Cloudways Flexible trial: https://support.cloudways.com/en/articles/10061830-how-to-subscribe-flexible-trial
- Cloudways brak root: https://support.cloudways.com/en/articles/5134090-why-can-t-i-have-root-access-to-my-server
- h2.nexus: https://h2.nexus/cli
- Oracle Always Free szczegółowy limit (aktualizacja 2026-06-12): https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm
- Oracle ogólna oferta (może być nieaktualna): https://www.oracle.com/pl/cloud/free/
- Google Cloud free: https://docs.cloud.google.com/free/docs/free-cloud-features
- Kamatera warunki: https://www.kamatera.com/faq/answer/what-are-the-terms-of-the-free-trial/
- Kamatera weryfikacja karty: https://www.kamatera.com/faq/answer/can-i-sign-up-for-the-free-trial-without-a-credit-card/
- OVHcloud trial: https://www.ovhcloud.com/pl/public-cloud/free-trial/
- Mikrus FROG: https://frog.mikr.us/

## Zalecana kolejność (bez marnowania okresów promocyjnych)

1. **Najpierw lokalnie/GitHub Actions**: zielony Rust CI,
   Supply Chain and Provenance, unit/integration i regresje; bez
   uruchamiania liczników promocyjnych.
2. **Bezpłatne, bardzo krótkie testy**: h2.nexus i VDS.GURU
   *wyłącznie jeśli regulamin i dostęp wymagany w teście to pozwalają*.
   Ich wyniki mogą potwierdzić Linux preflight, ale nie zastępują
   hostowego badania akceptacyjnego.
3. **Trwała warstwa 0 PLN**: Oracle Always Free, o ile tworzenie
   ARM VM jest możliwe. Wyróżnić ARM64 vs x86_64 w evidence; nie
   przemycać prób ARM do x86_64 gate.
4. **Właściwy etap 0.38**: wybrany OVHcloud Public Cloud x86_64,
   opcjonalnie osobny host Kamatera, albo istniejący IBM Cloud VSI.
   Aktywować 30-dniowe kredyty dopiero po przygotowaniu obrazu,
   inspekcji cen i planie likwidacji zasobów. Jeden dostawca
   nie jest wystarczającym dowodem `cross-cloud` failover.
5. **Etap 0.39**: najpierw potwierdzenie `/dev/kvm` i warunków
   nested virtualization; dopiero potem test Firecracker. Sam
   podpis `KVM` w nazwie VPS nie rozwiązuje tego wymagania.

## Bezwzględna bramka bezpieczeństwa

Nie deployować prawdziwych certyfikatów, kluczy podpisywania,
tokenów IAM, danych klientów ani konfiguracji produkcyjnych na
anonimowych, 15-minutowych i niezweryfikowanych hostach.
Wszystkie testy z ruchem sieciowym wykonywać wyłącznie
w autoryzowanej infrastrukturze, na kontrolowanych
syntetycznych tenantach. Operator musi potwierdzić warunki
promocji i billing; **promocja nie oznacza limitu wydatków**.

Preflight: `python3 scripts/ibm_cloud_wasi_preflight.py`
(pomimo nazwy skrypt nie korzysta z IBM API; sprawdza Linux).
Do finalnego odbioru: `scripts/wasmtime_acceptance.sh`,
`scripts/wasi_host_acceptance.py`,
`docs/STAGE_038_OPERATOR_SIGNOFF.md`.

**Status decyzji: 0.38 PARTIAL / PRODUCTION NO-GO**, niezależnie
od liczby oferowanych okresów próbnych. Wymagane są rzeczywiste,
śledzalne logi testów i podpis audytora.
