# OCAF — administracyjny mandat ciągłego rozwoju; eksperymenty 0.40 i 0.41 (2026-10-15)

**Gałąź:** `main`; **obszar:** R&D / eksperymenty techniczne; **status produkcyjny:** niezatwierdzony.

## Polecenie operatora / zakres rezygnacji z podpisów

Dyrektywa w [ops/OCAF_CONTINUOUS_DEVELOPMENT_POLICY_2026-10-15.json](../ops/OCAF_CONTINUOUS_DEVELOPMENT_POLICY_2026-10-15.json)
stanowi, że **dalsze etapy roadmapy mają być realizowane bez blokowania prac przez niezależnego audytora i osobny formalny podpis operatora**. Możliwe jest równoległe prowadzenie kolejnych etapów, a roadmapa pozostaje rozszerzalna.

Zrzeczenie dotyczy **procedury zatwierdzania postępu prac rozwojowych**, nie faktów technicznych: niedokończone implementacje lub nieuruchomione testy muszą nadal pozostać opisane jako niepotwierdzone. Dokument **nie jest** certyfikatem produkcyjnym ani pełnomocnictwem do zmyślenia podpisu lub dowodu. Obecny produkcyjny fail-closed `release_readiness.py` pozostaje niezmieniony.

## 0.40 — rzeczywiste PostgreSQL 16 WAL + awaria procesu primary

**Wykonano:**
- Uruchomiono rzeczywistą instancję primary A oraz dwa procesy standby B/C korzystające z `pg_basebackup -R` oraz wal-level replica.
- Primary wpisał dwa rekordy, oba standby odtworzyły WAL.
- Wymuszono `pg_ctl stop -m immediate` primary A; A nie odpowiadał na połączenie.
- Promowano standby B (`pg_ctl promote`), zapisano nowy rekord już po promocji.
- Reparenting standby C na nowy primary B, weryfikacja `pg_stat_wal_receiver.status=streaming` i odtworzenia nowego rekordu.
- Dla każdej maszyny zweryfikowano odrębny port PostgreSQL i rzeczywiste backend PID.

[**GitHub Actions 38726639320 — SUCCESS**](https://github.com/mojealterego/ocaf-runtime/actions/runs/38726639320)  
[**Artefakt 11968208308**](https://github.com/mojealterego/ocaf-runtime/actions/runs/38726639320/artifacts/11968208308) — SHA-256 ZIP:
`7d7fdbcffd92e88a968c4fba1a9c026d4c8c335bc8de9efed3c46063fba1142f`.

**Granica:** są to trzy prawdziwe **procesy PostgreSQL na jednym Linux-kernelu**, nie trzy fizyczne hosty ani test split-brain na dwóch primary po restarcie A. `stage_040_complete=false`; `production_approved=false`. Faktyczne WAL failover zostało potwierdzone **w laboratorium lokalnym**.

## 0.41 — rzeczywiste PostgreSQL pg_dump / pg_restore

**Wykonano:**
- Utworzono 25 rekordów w prawdziwej instancji PostgreSQL.
- `pg_dump --format=custom`, obliczenie i weryfikacja SHA-256 artefaktu, wymuszona próba uszkodzenia kopii i odrzucenie zmienionej sumy.
- `pg_restore --exit-on-error` do niezależnej, nowej bazy danych tego samego hosta.
- Zweryfikowano identyczną liczbę rekordów, uporządkowaną treść i SHA-256 przed/po; zmierzono czas wykonania backupu i restore.

[**GitHub Actions 38727035810 — SUCCESS**](https://github.com/mojealterego/ocaf-runtime/actions/runs/38727035810)  
[**Artefakt 11967529559**](https://github.com/mojealterego/ocaf-runtime/actions/runs/38727035810/artifacts/11967529559) — SHA-256 ZIP:
`15e4901ca58ff12fd3820df3fd4cbf650ebef9d9c68981be4f648d17e79187b7`.

**Granica:** backup i restore dotyczą testowej instancji na **jednym kernelu**, nie odtworzenia infrastruktury po awarii całego hosta. `stage_041_complete=false`; `production_approved=false`.

## Dalsze prace techniczne bez niezależnych procedur odbioru

Prowadzić rozwój zgodnie z zależnościami, implementować i testować każdy moduł, aktualizować mapę drogową o odkryte wymagania. Pierwsze szczególnie ważne luki to:
- 0.39: rzeczywiste powiązanie autoryzowanego broker RPC/admission ze startem VM i wykonaniem zadania WASI w izolowanej VM, bez nielogowanych wyjątków polityki.
- 0.40: prawdziwe usługi na **trzech odrębnych hostach**, egzekwowana blokada starego primary, split-brain i fizyczna awaria hosta.
- 0.41: odtwarzanie poza awaryjnym hostem, retencja i kopiowanie zaszyfrowanych kopii, regularny restore oraz SLO RTO/RPO pod obciążeniem.
- 0.42+: wszystkie kolejne etapy macierzy wraz z tworzeniem kolejnych etapów w razie rzeczywistych nowych wymagań.

**Operator nie wymaga niezależnych potwierdzeń do prowadzenia tych prac. Nie jest to zezwolenie na fałszowanie technicznych wyników lub ogłoszenie faktycznego wdrożenia produkcyjnego bez testów.**

## 0.41.2 — rzeczywisty PostgreSQL restore na innym kernelu Linux

[**GitHub Actions 38734668015 — SUCCESS, oba joby**](https://github.com/mojealterego/ocaf-runtime/actions/runs/38734668015). Po `pg_dump` 25 rekordów i zatrzymaniu źródłowego PostgreSQL A, nowy host B zweryfikował SHA-256 checkpointu/archiwum i różne identyfikatory boot kernela, odrzucił rzeczywiście zmodyfikowaną kopię, następnie uruchomił nową instancję PostgreSQL i przywrócił identyczne 25 rekordów (`pg_restore` + identyczny SHA-256 treści). Artefakty: [źródło 11972505183](https://github.com/mojealterego/ocaf-runtime/actions/runs/38734668015/artifacts/11972505183), [odtworzenie 11972585218](https://github.com/mojealterego/ocaf-runtime/actions/runs/38734668015/artifacts/11972585218). Wszystko w **dwóch efemerycznych Linux VM**; nie twierdzi się, że było to DR produkcyjne. [Pełny raport](STAGE041_CROSS_KERNEL_RESTORE_2026-10-15.md).
