# 0.42 — zbieranie dowodów jednego kandydata

Kontrola workspace obejmuje cztery aktualnie zadeklarowane pakiety:
`ocaf_runtime`, `ocaf_stage_ledger`, `ocaf-transport-crdt`, `ocaf-g01-swarm`.
Nie oznacza to ukończenia wszystkich planowanych agentów lub rojów.

Generator cargo-cyclonedx tworzy osobny dokument dla każdego pakietu.
G01 jest samodzielną aplikacją, więc nie występuje w zależnościach runtime.
`assemble_workspace_sbom.py` zachowuje jego komponenty, licencje i graf
zależności w zbiorczym dokumencie. Prefiksy identyfikatorów zapobiegają
kolizjom, a graf głównego pakietu nie dostaje sztucznej zależności do G01.
Zbiorczy dokument przechodzi istniejącą kontrolę i jest przedmiotem
atestacji SBOM w pipeline supply-chain.

`scripts/stage042_release_decision.py` pobiera przez GitHub API wskazane
przebiegi workspace, supply-chain, rollbacku podpisanych binariów oraz
rollbacku migracji i stanu kolejki. Wszystkie muszą dotyczyć
tego samego pełnego SHA, repozytorium i oczekiwanej ścieżki workflow na main.
Brakujące, trwające lub nieudane próby są jawnie zapisane jako blokady.
Skrypt zapisuje nowy plik JSON, nie nadpisuje istniejącego odbioru.

Sukces CI jest dowodem zakresu konkretnego workflow. Nie zastępuje
odbioru pełnego stanu runtime, wszystkich dostawców sekretów ani produkcji.
Obecny kolektor pozostawia NO_GO i production_approved=false dla tych
niezamkniętych kryteriów. Rezygnację właściciela z niezależnego audytu
odnotowuje osobno; nie tworzy podpisu właściciela.

Długie testy 0.40/0.41 uruchomione na wcześniejszych SHA nie mogą być
automatycznie przedstawiane jako testy wybranego kandydata 0.42.
