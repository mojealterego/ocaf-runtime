# 0.39 — zamknięcie decyzją właściciela na podstawie symulacji

Właściciel 2026-10-26 polecił oznaczyć etap 0.39 za zakończony i kontynuować
etap 0.40. Wcześniej wybrał symulację, ponieważ nie ma dostępu do dwóch
trwałych hostów KVM. Decyzja zamyka **kolejkę projektową 0.39**, dzięki czemu
prace nad 0.40 mogą być prowadzone bez blokady administracyjnej.

Podstawą jest [podpisany raport próby zastępczej](STAGE039_SIMULATED_TWO_RUNNER_ATTESTATION_2026-10-26.md)
oraz [dwutenantowy test guest kprobe/Aya/WASI na KVM](https://github.com/mojealterego/ocaf-runtime/actions/runs/40262951835).
Próba zastępcza obejmuje dwa efemeryczne runnery z 100 cyklami Firecracker
każdy, odtworzenie checkpointu przez artefakt na drugim runnerze i partycję
mTLS między dwiema przestrzeniami sieciowymi jednego jądra.

Stan kanoniczny: `CLOSED_BY_OPERATOR_SIMULATION_TECHNICAL_OPEN`.
Oznacza to **zakończenie administracyjne**, nie ukończenie technicznej bramki
trwałych hostów i nie zgodę produkcyjną. Dowody uruchomieniowe zachowują
`stage_039_complete=false` i `production_approved=false`, ponieważ nie
przeprowadzono bezpośredniej partycji między dwoma trwałymi hostami ani
żywego failover brokera. Delegowany podpis symulacji nie jest osobistym
podpisem właściciela wymaganym w [formalnym dossier](STAGE039_OWNER_SIGNOFF.md).

Zadania techniczne można uzupełnić po uzyskaniu trwałych hostów. Ich brak nie
blokuje odtąd prac projektowych nad 0.40; także w 0.40 wyniki laboratoryjne
muszą być odróżniane od rzeczywistej replikacji WAL między hostami.
