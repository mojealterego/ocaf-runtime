# OCAF 0.38 — 30-dniowy pilotaż OVHcloud Public Cloud (warunkowy)

**Status: PLAN / NOT DEPLOYED / PRODUCTION NO-GO.** Nie aktywowano
promocji, nie utworzono zasobów i nie przeprowadzono testów na
OVHcloud w ramach tego dokumentu. Wszystkie zmiany kodu OCAF
muszą trafić wyłącznie na gałąź `main`.

## Podstawa oferty i zasady bezpieczeństwa kosztowego

Oficjalna promocja OVHcloud dla **pierwszego projektu Public Cloud**
oferuje kwalifikującym się klientom **1000 PLN na 30 dni**.
Aktywacja projektu rozpoczyna okres ważności, nie samo planowanie.
Konieczna jest zarejestrowana metoda płatności. Po wyczerpaniu
kredytu albo po okresie próbnym działające usługi mogą obciążać
tę metodę płatności. OVHcloud VPS, klasyczne serwery dedykowane
i pozostałe produkty *poza Public Cloud* nie są automatycznie objęte
tym voucherem. Nie zakładaj, że każdy typ instancji w katalogu
Public Cloud jest kwalifikowany bez sprawdzenia koszyka.

- https://www.ovhcloud.com/pl/public-cloud/free-trial/
- https://docs.ovhcloud.com/pl/guides/public-cloud/cross-functional/create-a-public-cloud-project
- https://docs.ovhcloud.com/pl/guides/public-cloud/cross-functional/increasing-public-cloud-quota

**Gate A — BEFORE ACTIVATION:** sprawdź prawo do promocji, projekt
pierwszego użycia, dostępne regiony, limity instancji, aktywne
promocje, warunki opodatkowania i rozliczeń, profil sprzętu oraz
koszyk. Aktywować wyłącznie wtedy, kiedy manifesty, skrypty,
obrazy i plan dostępu SSH/VPN są przygotowane.

Jeżeli aktywacja nastąpi **14 października 2026**, planowy
koniec okna 30 dni wypada około **13 listopada 2026**.
Dokładny termin ważności i strefę czasową należy odczytać
z panelu klienta; w kalendarzu operatora ustawić przegląd
kosztów przed końcem okresu i zaplanować **zakończenie oraz
usunięcie zasobów odpowiednio wcześniej**, nie o ostatniej
minucie. W razie późniejszej aktywacji wszystkie terminy są
liczone od faktycznej daty aktywacji.

### Wewnętrzne limity pilotażu (propozycja, nie prognoza cennika)

| Koperta | Limit orientacyjny | Warunek |
| --- | ---: | --- |
| Jeden host OVH Linux x86_64, Wasmtime, cgroup v2 | 400 PLN | Kosztorys i testy 0.38 |
| Drugi host i testy rozproszone/mTLS/fencing | 250 PLN | Po zielonym pierwszym hoście |
| Opcjonalny krótki eksperyment z `/dev/kvm` | 150 PLN | Tylko gdy profile/warunki pozwalają i po uzgodnieniu ryzyka |
| Rezerwa na networking, storage, rozliczenia | 200 PLN | Unikać wykorzystania do końca |
| **Suma** | **1000 PLN** | **Nie jest to limit billingowy** |

**Operator stop rule:** nie rozpoczynaj nowej operacji, jeśli
wycena do końca 30-dniowego okna wraz z aktywnymi zasobami
może przekroczyć niewykorzystaną kwotę vouchera lub wewnętrzny
limit, nawet jeśli panel pokazuje tylko bieżące zużycie.
Do pełnego planowania kosztów wymagane jest uwzględnienie
instancji, volumes, snapshots, public IP, gateway, transferu,
Object Storage i aktywnych zewnętrznych usług.

## Architektura 0.38: OVH + IBM Cloud

- **OVHcloud Public Cloud**: wydzielone, krótkotrwałe staging;
  1–2 Linux x86_64, docelowo co najmniej 4 vCPU i 8–16 GiB RAM
  zależnie od wyceny i dostępnych flavorów. Wdrożenie systemd
  i cgroup v2, syntetyczne tenanty, ograniczony ruch sieciowy.
- **IBM Cloud**: niezależny staging, długoterminowy punkt odniesienia;
  decyzja o użyciu kredytów USD osobno, bez konwersji saldo/PLN.
- **GitHub `main`**: źródło prawdy dla dokładnego SHA,
  zaszytych limitów, testów i ścieżki evidence. Bez tworzenia
  oddzielnych branchy.
- **Brak otwartego RPC**: porty koordynatora/Zenoh udostępniane
  wyłącznie poprzez kontrolowaną sieć prywatną lub zatwierdzony
  tunel. mTLS z kluczami nieumieszczanymi w kodzie.
- **Cross-cloud**: porównać wyniki WASI, limity procesów, różnice
  systemd/cgroup, opóźnienia i przeżywalność restartu; nie łączyć
  produkcyjnych tenantów w testowej sieci.

## Harmonogram względem momentu aktywacji

| Okno | Cel | Dowód |
| --- | --- | --- |
| D0–D2 | Preflight, kontrola kosztów, SSH/VPN, zatwierdzenie profilu | Read-only JSON i aprobaty operatora |
| D3–D9 | WASIp1 real Wasmtime: sigkill, timeout, RSS, FD, pids, 100 cycles | `wasi-038-host.json`, komplet exit codes |
| D10–D17 | 2 hosty: mTLS, tenant isolation, network partition, fencing, restart | Metryki cgroup i raporty negatywnych przypadków |
| D18–D23 | Cross-cloud z IBM, porównanie niezawodności i śladów audit | Raport porównawczy bez sekretów |
| D24–D26 | Triage, regresja, podpisy operatora i niezależny audyt | Odbiór 0.38, jeśli kryteria spełnione |
| D27–D29 | Eksport dowodów, zaplanowane usunięcie wszystkich billable resources | Lista usuniętych zasobów i przegląd fakturowania |

**Nie uruchamiaj tego planu automatycznie.** Harmonogram jest
krótszy niż pełne 30 dni celowo, aby pozostawić bufor na
rozliczenia i opóźnione usuwanie zasobów.

## Linux read-only preflight na instancji

Po autoryzowanym stworzeniu VSI Public Cloud, bez kluczy w GitHub:

```bash
git clone https://github.com/mojealterego/ocaf-runtime.git
cd ocaf-runtime
git rev-parse HEAD
python3 scripts/ibm_cloud_wasi_preflight.py \
  --report ./ovhcloud-038-preflight.json
```

Nazwa istniejącego skryptu jest historyczna: sprawdza **standardowe
mechanizmy Linuksa**, nie kontaktuje się z API IBM ani OVH.
Do dowodów użyj
`scripts/wasmtime_acceptance.sh` i
`scripts/wasi_host_acceptance.py` zgodnie z
[protokół odbioru](STAGE_038_OPERATOR_SIGNOFF.md).
Weryfikuj bieżący `main` oraz status **obu** workflow w
GitHub Actions na dokładnym SHA.

## Etap 0.39 — `/dev/kvm`, microVM i Firecracker: eksperyment, nie założenie

OVHcloud dokumentuje udostępnianie VMX na **niektórych instancjach
Public Cloud**, ale zastrzega, że **live migration może wywołać
kernel panic podczas zagnieżdżonej wirtualizacji**. Dlatego
`vmx`/`svm` lub sam `/dev/kvm` nie wystarczy do uznania
instancji za produkcyjnie dopuszczoną do hostowania Firecracker.

- Na jednym hostcie testowym można **osobno** zweryfikować
  `egrep '(vmx|svm)' /proc/cpuinfo`,
  `test -c /dev/kvm` i autoryzację dostępu do urządzenia.
- Przed dłuższym microVM/KVM wymagane są warunki kompatybilności
  profilu, możliwość live migration, dowód trybu awaryjnego
  i niezależna aprobata. Nie uruchamiać testów microVM na
  tym samym komponencie, na którym wykonywane są
  końcowe dowody odbiorowe 0.38.
- Metal Instances **w ofercie Public Cloud** są potencjalną
  alternatywą, lecz ich dostępność, kwalifikacja do kredytu
  oraz rozliczenia należy zweryfikować w konkretnym
  projekcie. Zwykły OVH Dedicated Server poza Public Cloud
  to inna usługa i inne warunki promocyjne.

- https://docs.ovhcloud.com/en/guides/public-cloud/cross-functional/faq-pci
- https://www.ovhcloud.com/en/public-cloud/regions-availability/

## Go / No-Go i teardown

**0.38 production NO-GO** dopóki nie przejdą pomiary na
rzeczywistym hoście, 2-tenants, network partition, failover,
signed manifest regression, testy negatywne, niezależny
security review i podpis operatora. Sam GitHub green nie
stanowi dowodu produkcyjnej izolacji.

**Exit gate:** pobrać audytowalne raporty poza OVH,
usunąć instancje, pozostałe volumes, snapshots, load
balancers, reserved/floating IP, object storage i inne
płatne zasoby, a następnie sprawdzić stan fakturowania.
Samo zatrzymanie systemu operacyjnego nie dowodzi
zakończenia naliczania kosztów. Ustawienia płatności
i alerty nie są gwarancją blokady nadwyżek.

**Nigdy** nie umieszczaj kart płatniczych, tokenów OVH,
kluczy SSH, prywatnych PEM ani credentiali IAM w
repozytorium, raportach publicznych lub treści czatu.
