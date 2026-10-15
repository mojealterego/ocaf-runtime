# Dyspozycja operatora — administracyjne zamknięcie prac 0.38 i start 0.39

Data: 2026-10-15. Repo: `mojealterego/ocaf-runtime`, wyłącznie `main`.

## Dyspozycja przekazana w rozmowie

Operator złożył wyraźne polecenie:

> Etap 0.38 uznaj za ukończony a wdrożenie produkcyjne odblokowane pracuj nad Etapem 0.39

**Decyzja administracyjna:** operator deklaruje zamknięcie prac nad backlogiem
0.38 i przejście do inżynierii 0.39. Ta decyzja jest zapisana jako
**`OPERATOR_DIRECTIVE_CLOSE_038_AND_START_039`**. Nie oznacza, że wszystkie
kryteria odbiorowe 0.38 uzyskały rzeczywiste wyniki PASS.

## Granice dowodowe

- Zakończono realne testy mTLS/WASI między dwiema maszynami **CI**,
  izolacji dwóch tenantów w środowisku **CI** oraz kontrolę łańcucha
  dostaw. Te runner'y nie są docelowymi trwałymi hostami.
- Wymagane A01–A10: brak kompletu poświadczonych testów z dwóch
  trwałych i odrębnych hostów systemd/cgroup v2.
- A11: e-maile od wskazanych uczestników potwierdzają wolę zatwierdzenia
  odbioru produkcyjnego, ale nie zawierają niezależnego raportu
  adwersarialnego. Wiadomości ani generowane przez automat podpisy
  techniczne nie są weryfikowanym raportem audytora.
- A12: dyspozycja operatora jest faktyczna, jednak końcowe
  poświadczenie pełnej macierzy dowodowej nie może wskazywać PASS,
  dopóki brak tych dowodów.

## Rejestr decyzji — rozdzielenie administracji od bezpieczeństwa

| Kwestia | Stan |
|---|---|
| Priorytet prac 0.38 | **Zamknięcie administracyjne na polecenie operatora** |
| Rozpoczęcie developmentu 0.39 | **DOZWOLONE I W TOKU** |
| Formalna walidacja 0.38 A01–A12 | **NIEKOMPLETNA** |
| Udokumentowany niezależny audyt | **NIEPRZEDŁOŻONY** |
| Domyślna automatyczna weryfikacja dowodowa 0.38 | **NO-GO; bez fałszowania A01–A12** |
| Autoryzacja administracyjna operatora do próby wdrożenia z odstępstwem | **ODBLOKOWANA na wyraźne żądanie operatora; testy pozostają niekompletne** |
| Faktycznie wykonane wdrożenie produkcyjne | **NIE POTWIERDZONO** |

Dyspozycja „odblokuj produkcję” pozostaje żądaniem operatora. Samo
jej wpisanie do repozytorium, zmiana roadmapy lub sygnatura kluczem
kontrolowanym przez automat nie daje podstaw do automatycznego ominięcia
kontroli runtime. `scripts/stage038_production_gate.py` pozostaje
nienaruszony i nadal wymaga autentycznych dowodów. Nie podmieniamy
wyników testowych ani tożsamości audytorów.

**Kontynuacja 0.39 jest dopuszczona; oznaczenie produkcji jako bezpiecznie
wdrożonej pozostaje niedopuszczalne bez właściwych dowodów.**


## Ponowne jednoznaczne polecenie operatora — odblokowanie release (2026-10-15)

Operator przekazał kolejną dyspozycję: **„Masz odblokować […] ja go zablokowałem i ja go odblokowuje i pracować nad Etapem 0.39”**.

Wprowadzone zmiany programowe:

- `ops/OCAF_RELEASE_OPERATOR_DECISION_2026-10-15.json` — jawny stan
  `"administrative_release_blocked": false`, decyzja
  `UNBLOCK_PRODUCTION_DEPLOYMENT` i jednocześnie
  `"verified_production_readiness": false` oraz `"production_deployed": false`.
- `scripts/release_readiness.py` — **rzeczywista, opt-in ścieżka**
  `--operator-override-policy ... --ack-unverified-production-risk`,
  która zwraca kod 0 i `administrative_release_unblocked: true`, ale
  nadal pokazuje wszystkie braki dowodowe i `production_approved: false`.
  Nie zmienia zachowania domyślnego bez jawnego parametru.
- `scripts/stage038_production_gate.py` pozostaje niezmieniony:
  wynik weryfikacji autentyczności dowodów pozostaje osobny od ryzykownej
  administracyjnej decyzji właściciela repo.

```bash
python3 scripts/release_readiness.py --repo . \
  --operator-override-policy ops/OCAF_RELEASE_OPERATOR_DECISION_2026-10-15.json \
  --ack-unverified-production-risk
```

Ta komenda **odblokowuje operatorową decyzję dla systemów świadomie
korzystających z tej ścieżki**, ale nie uruchamia wdrożenia, nie modyfikuje
zewnętrznych środowisk GitHub Environments ani nie jest poświadczeniem
prawdziwego audytu bezpieczeństwa. Operator akceptuje brak formalnych
dowodów na własne ryzyko; inne systemy automatycznie egzekwujące
A01–A12 mogą nadal odmówić wdrożenia.

**Rozwój 0.39 trwa niezależnie od stanu produkcyjnego 0.38.**
