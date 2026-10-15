# Próba znalezienia darmowego hosta Linux x86_64 KVM/Aya dla OCAF 0.39

Data: 2026-10-15. Gałąź: main. **Host: nieuruchomiony / niezweryfikowany.**

## Zweryfikowany kandydat

**CircleCI Android Linux machine, image android:current, resource_class
medium.gen2** — CircleCI oficjalnie opisuje wsparcie nested
virtualization w tej rodzinie obrazów.
Źródło: https://circleci.com/docs/guides/execution-managed/android-machine-image/

W repozytorium istnieje konfiguracja `.circleci/config.yml`,
która pozwala utworzyć **rzeczywistą tymczasową maszynę CI** na
CircleCI (bez rejestracji nowego płatnego dostawcy).
Nowe zadanie `stage039-hardware-nested-kvm` uruchamia
próby `KVM_GET_API_VERSION` i `KVM_CREATE_VM`, kompiluje
loader Aya i wykonuje negatywny test ruchu dwóch dedykowanych
cgroup z oddzielną ścieżką powrotu po detach. Raporty zapisywane
w artefakcie `ocaf039-circleci-real-kvm-aya`.

Uruchomienie zadania wymaga, by CircleCI było autoryzowane
dla repozytorium `mojealterego/ocaf-runtime` i udostępniało
plan z kredytami pozwalającymi na użycie Android machine.
CircleCI Free oferuje 30 000 kredytów na miesiąc; oficjalna
dokumentacja opisuje dodatkową alokację dla kwalifikujących
się publicznych projektów open-source:
https://circleci.com/docs/guides/plans-pricing/credits/
Status osobistego konta i dostępny limit kredytów **nie został
zweryfikowany**. Nie obciążono umyślnie płatnego konta.

## Próba wyzwolenia

W commicie `d3a406d651b3bbcb0fbb5d21260c315f1b6b5d0e`
tymczasowo włączono domyślny flag `run_stage039_kvm: true`.
Samo push do GitHub nie dowodzi, że CircleCI jest połączone:
nie pojawiła się potwierdzona kontrola statusu CircleCI przy
tym commicie. Przywrócono domyślny flag `false`, żeby kolejne
commity nie zużywały potencjalnie nieograniczonych kredytów.
Nie używano metered web automation ani nie kupiono serwera.

Gdy konto CircleCI jest podłączone, należy wyzwolić pipeline dla
`main` z `run_stage039_kvm=true`. Nie wymaga to przekazywania
haseł, prywatnych tokenów, żadnych kluczy w rozmowie.

## Dlaczego nie poprzednie środowiska

- GitHub-hosted Ubuntu 24.04: rzeczywisty `KVM_CREATE_VM` NO-GO
  w dotychczasowych workflow.
- Sprite `ocaf-stage038-linux`: `/dev/kvm` nie istnieje, PID 1
  to `tini`; nie jest hostem systemd KVM.
- DigitalOcean: brak istniejących Dropletów, a nowy jest
  standardowo płatny, niezależnie od jego KVM capability.
- Render i Railway: serwisy aplikacyjne w kontenerach, brak
  pozytywnego testu `/dev/kvm`/pełnego systemd hosta.
- Oracle Always Free E2/A1: brak udowodnionej obecności KVM
  wewnątrz instancji, nie dopuszczać jako hosta bez probe.

## Bramka 0.39

Dopiero raport `KVM_CREATE_VM=true` plus realne
`BPF_PROG_LOAD`, przypięcie programu do cgroup A, odmowa połączenia
A, pozytywne połączenie B i odzyskanie A po detach stanowią
sprzętowe dowody negatywnej izolacji sieciowej. Osobno potrzebny
jest za-jailer uruchomiony i poświadczony gość Firecracker na
tej samej kompatybilnej architekturze.

**Aktualny wynik 0.39: NO-GO dla realnej maszyny Firecracker i Aya
na hoście, bo nie ma potwierdzonej egzekucji na zgodnym hostcie.**
