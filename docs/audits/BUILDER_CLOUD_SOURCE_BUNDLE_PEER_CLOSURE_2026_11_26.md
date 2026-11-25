# OCAF Agents Builder Cloud — zamknięcie zależności paczki źródłowej

Data: 2026-11-26. Gałąź: `main`. Commit źródłowy: `2f16219c36c65a9830f350e090dfb12ed555794c`.
Obszar: 0.52, AB-CLOUD-CODE/NOCODE; GATE-03/04/05/08.

## Ustalenie RED

`scripts/builder_cloud_ingress.py` zawiera `from scripts.builder_cloud_peer import verify_bridge_peer`. W commitcie źródłowym selektor `scripts/package_agents_builder.py::select_builder` wykluczał `scripts/builder_cloud_peer.py` i `scripts/tests/test_builder_cloud_peer.py` z *focused source ZIP*. `REQUIRED_BUILDER_FILES` również ich nie wymagał; test archiwum i wyzwalacz workflow nie obejmowały zależności. Wynik: paczka mogła mieć poprawny checksum i jednocześnie nie zawierać importowanego modułu.

## Zmiana

- Dodano importowany moduł i testy do `EXACT` oraz `REQUIRED_BUILDER_FILES` w istniejącym skrypcie pakowania.
- Wymuszono te pliki przez `ANCHORS` testu `test_builder_bundle_platform_coverage.py`, który w istniejącej procedurze sprawdza ZIP/manifest SHA-256 i odmowę publikacji przy braku kotwicy.
- Dodano regresję do `test_package_agents_builder.py`.
- Poszerzono wyzwalacze odpowiednich workflow, aby zmiana importowanej zależności nie była pomijana przez CI.

## Dowody i ograniczenia

Sprawdzono rzeczywiste źródła importu, brak zależności w listach sprzed zmiany oraz kompletność ścieżek na poziomie statycznym. Testy Python/Rust, ekstrakcja i uruchomienie z gotowego ZIP, GitHub Actions oraz produkcyjne wdrożenie **nie zostały wykonane ani zaliczone w tej operacji**; należy zweryfikować je na czystym checkout danego SHA. Bezpieczne komendy:

```sh
python3 -B -m unittest discover -s scripts/tests -p test_package_agents_builder.py -v
python3 -B -m unittest discover -s scripts/tests -p test_builder_bundle_platform_coverage.py -v
python3 -B -m unittest discover -s scripts/tests -p 'test_builder_cloud_*.py' -v
bash scripts/verify_builder_cloud_stack.sh
```

To naprawa kompletności plików źródłowych, nie pełna akceptacja Cloud Buildera, 0.52 ani projektu. `production_approved=false` pozostaje w mocy. Nie dotyczy wykonywania płatności i nie odblokowuje OSTc/PQC.
