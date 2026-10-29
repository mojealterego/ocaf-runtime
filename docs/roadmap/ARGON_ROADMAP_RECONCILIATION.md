# Roadmapa Argona — uzgodnienie z main

Źródło: `ocaf-runtime_ROADMAP(1).PDF.pdf`, 78 stron, Drive ID `1JQ-puwrgu_bS4_qb4jY3XvezssJLhSRW`. Tekst i sumy kontrolne: `../sources/drive-ocaf/manifest.json`.

W nagłówku wydruku GitHub widnieje commit `5a6ebf0c227978d40a14b02232e9f0379df1c82d`. Git potwierdza, że jest przodkiem aktualnego main. PDF jest historyczną migawką kanonicznego ROADMAP.md; ekstrakcja PDF nie jest porównaniem bajtowym źródła Markdown.

## Co zmieniło się od migawki

Porównanie źródła ROADMAP.md z Git dla `5a6ebf0` i `aac6458`:

- Dopisano etap 334: aktywacja SASOS z realnymi backendami, testami i dowodami.
- Liczba pozycji wzrosła z 407 do 408; następny nowy numer zmienił się z 334 na 335.
- Etap 334 ma opis częściowej implementacji i ograniczone wyniki CI: capability/adaptery/Z3, Zenoh SHM i Firecracker/KVM. Nie stanowi to odbioru wszystkich backendów.
- Dodano rejestr 45 grup wymagań i produktów z czterech dokumentów Drive.
- Historyczne definicje 0.50–0.55 i dziesięć produktów Agents Builder są zachowane; PDF również opisuje je jako NOT_IMPLEMENTED.

## Zasada dalszej realizacji

Kanoniczny ROADMAP.md w main pozostaje listą etapów. Roadmapa Argona potwierdza jej pochodzenie. Pozostałe PDF-y dostarczają wymagań; nie zastępują historycznych numerów ani wyników testów. Rejestr `DRIVE_IMPLEMENTATION_BACKLOG.md` wskazuje pozostałe prace i proponowane przypisania semantyczne. Ta migawka nie wprowadza nowych zadań ani nie podnosi żadnego statusu na DONE.
