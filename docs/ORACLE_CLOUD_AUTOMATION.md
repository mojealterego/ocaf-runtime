# Oracle Cloud — kontrolowana automatyzacja OCAF

**Stan:** działający lokalnie kontroler read-only i testy offline; brak podłączenia do konta OCI, brak zmian rozliczeń i brak wdrożonych serwerów. Zmiany repozytorium wyłącznie na gałęzi main.

## Zakres strony wskazanej przez użytkownika

Strona: https://cloud.oracle.com/invoices-and-orders/upgrade-and-payment?region=eu-frankfurt-1

To ekran **Upgrade and Payment**, a nie panel uruchamiania instancji Compute. Wymaga zalogowanej sesji. Parametr eu-frankfurt-1 nie dowodzi, że Frankfurt jest regionem macierzystym (home region) konta.

Automatyzacja finansowa jest domyślnie blokowana. Nie wolno bez odrębnego, szczegółowego upoważnienia klikać Upgrade/Confirm/Pay, zatwierdzać Pay As You Go, edytować metody płatności ani generować jakichkolwiek kosztów. Użytkownik sam loguje się, podaje MFA i zatwierdza płatności. Nie pobieramy danych kart, kluczy ani tokenów do kodu, logów lub rozmowy.

## Uruchomiony moduł

Plik: scripts/oracle_cloud_readonly.py

Dostępne operacje:

1. **plan**: wykonuje się całkowicie offline. Proponuje dwa serwery ARM Ampere A1 po 1 OCPU, 6 GB RAM i 50 GB boot każdy. Odrzuca łączny plan przekraczający 2 OCPU, 12 GB RAM, 200 GB boot. Nie tworzy zasobów i nie sprawdza zajętości kwoty przez już istniejące instancje.
2. **inspect**: działa po autoryzacji w OCI Cloud Shell. Tylko cztery odczyty za pomocą oficjalnego OCI CLI: subskrypcje i home region, wykaz instancji w określonym compartment, wykaz VCN oraz wykaz budżetów. Raport zawiera liczności i sumy, nie pokazuje OCID, nazw, adresów IP, surowych odpowiedzi API ani kluczy.
3. Każda nieudana operacja odczytu przerywa program. Żadnych wywołań create, launch, update, delete ani upgrade.

Stan płatności i pozostałych kredytów jest raportowany jako **not_verified_by_this_tool**. Nie wolno przedstawiać wyniku jako potwierdzenia salda Oracle czy rodzaju subskrypcji.

## Offline: uruchomienie bez konta

~~~bash
cd ocaf-runtime
python3 scripts/oracle_cloud_readonly.py plan
python3 -m unittest discover -s scripts/tests -p test_oracle_cloud_readonly.py -v
~~~

## OCI Cloud Shell: odczyt rzeczywistego konta

Użytkownik loguje się samodzielnie do Oracle Cloud i otwiera Cloud Shell. Instaluje repozytorium lub otwiera już istniejące.

~~~bash
git clone https://github.com/mojealterego/ocaf-runtime.git
cd ocaf-runtime
read -r -p 'Tenancy OCID: ' OCI_TENANCY_OCID
read -r -p 'Compartment OCID: ' OCI_COMPARTMENT_OCID
python3 scripts/oracle_cloud_readonly.py inspect \
 --tenancy-id "$OCI_TENANCY_OCID" \
 --compartment-id "$OCI_COMPARTMENT_OCID" \
 --region eu-frankfurt-1 \
 --output ./ocaf-oracle-inventory-private.json
~~~

Raport tworzony jednorazowo z prawami 0600, bez nadpisywania. Nie przesyłać go do publicznych repozytoriów. Konto musi posiadać wymagane uprawnienia read-only dla listowania budżetów, sieci, instancji i regionów. Odmowa IAM kończy się błędem, nie fałszywym potwierdzeniem pustego konta.

## Bramka ograniczeń

- Sama możliwość użycia kształtu A1 nie potwierdza, że jest darmowy na danym koncie ani że dostępne są hosty.
- Rzeczywisty home region należy ustalić osobno.
- Raport dotyczy tylko jednego wskazanego compartment: pozostają do sprawdzenia inne compartmenty, boot volumes, block storage, backupy, sieci i obecne opłaty.
- OCI Budgets to miękkie powiadomienia, **nie twarda blokada kosztów**.
- Obecny test Wasmtime 49.0.2 w OCAF jest przypięty do Linux x86_64; serwer Ampere A1 ARM64 nie spełnia automatycznie tego kryterium odbioru 0.38.
- Konto Oracle Always Free nie wymaga przejścia na Pay As You Go tylko dla darmowych zasobów. Żadnej aktualizacji płatności bez osobnego potwierdzenia właściciela.

## Dalszy odbiór

Testy offline: scripts/tests/test_oracle_cloud_readonly.py (10 przypadków). GitHub Rust CI ma już Python unittest discovery.

Po uzyskaniu sesji właściciela można wykonać read-only inspekcję witryny i konta. Etap 0.38 pozostaje PARTIAL / PRODUCTION NO-GO do momentu rzeczywistych testów hosta i niezależnego podpisu odbioru.

### Źródła OCI

- https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier.htm
- https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm
- https://docs.oracle.com/en-us/iaas/tools/oci-cli/latest/oci_cli_docs/cmdref/iam/region-subscription/list.html
- https://docs.oracle.com/en-us/iaas/tools/oci-cli/latest/oci_cli_docs/cmdref/budgets/budget/budget/list.html
- https://docs.oracle.com/en-us/iaas/Content/Billing/Concepts/budgetsoverview.htm
