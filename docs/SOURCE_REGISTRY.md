# Rejestr źródeł i załączników

Rejestruje źródła otrzymane do implementacji. Hash pozwala rozpoznać dokładny plik, a status rozdziela wymagania od dowodów wykonania.

Pełna kontrola pięciu załączników (hash, 4 strony PDF, ekstrakcja tekstu i mapa
stron) znajduje się w [`SOURCE_REGISTRY_ATTACHMENTS_2026-10-21.md`](SOURCE_REGISTRY_ATTACHMENTS_2026-10-21.md).

| Plik | SHA-256 | Zakres | Status użycia |
|---|---|---|---|
| `Instrukcje dla Agenta (2).md` | `27dcaa7626e39950e50f1321944c30ba5bee8c981a9b8b4e4fb2dfa9d04f3048` | instrukcje pracy i oczekiwane materiały | wymagania |
| `02_SPECYFIKACJA...md` | `5ba7ec42d84491fcce851c7fdb33209609c98736aaf6b5884f569635a5bbc39c` | specyfikacja OCAF/OMAs/OMAt/OSTc | wymagania |
| `02_SPECYFIKACJA...pdf` | `eab9cdb5d47de300c7f6629786764e6c672096492a83282cfc8d4f1373a1ede6` | kopia PDF, 4 strony | wymagania |
| `01_PRZYKLADOWA_DOKUMENTACJA...md` | `cd87cc14262b4f0e2ff77f23e6c33e13bf441f4719112de6ddefee1ed2e2a49f` | przykładowy pakiet dokumentacji | wzór, nie certyfikat |
| `01_PRZYKLADOWA_DOKUMENTACJA...pdf` | `93414ac0cfd02ae92f0e5f4d242b986dc1623ffdc789d9685ebeba9a92ae0108` | kopia PDF, 4 strony | wzór, nie certyfikat |
| `docs/roadmap/agents_builder_products_v11.json` | repozytorium | 10 wariantów Agents Builder | rejestr zakresu |

## Zasady interpretacji

- Dokument przykładowy nie ustanawia certyfikacji, podpisu, ceny ani stanu produkcyjnego.
- Każde twierdzenie o ukończeniu musi wskazywać repozytorium, commit, digest artefaktu i wynik testu.
- OSTc jest wyłączone do czasu dostarczenia pełnej specyfikacji; nie wolno udawać szyfrowanego dostępu do źródeł.
