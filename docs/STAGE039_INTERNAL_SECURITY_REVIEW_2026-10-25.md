# 0.39 — wewnętrzny przegląd granicy obraz–VMM

Data: 2026-10-25. Zakres: ścieżka uruchomienia rzeczywistego Firecracker w
`scripts/stage039_live_firecracker.py`, ze szczególnym uwzględnieniem
podmiany obrazu rootfs pomiędzy weryfikacją manifestu i otwarciem go przez
proces VMM. Jest to przegląd wewnętrzny; właściciel odstąpił od niezależnego
audytu. Nie jest to test ucieczki z gościa ani zgoda produkcyjna.

## Ustalenia i zmiana

Poprzednia funkcja `attest_running_rootfs_fd` obliczała SHA-256 z *nazwy*
pliku w chroot, po czym sprawdzała, czy VMM ma FD na inode o takim samym
numerze i urządzeniu. Między tymi operacjami możliwa była zmiana bajtów tego
samego inode: zgodność numeru inode nie dowodzi zgodności jego zawartości.

Funkcja teraz otwiera `/proc/<pid>/fd/<n>` dla faktycznego deskryptora VMM,
oblicza SHA-256 z jego bajtów i porównuje go z przypiętym hashem manifestu.
Sprawdza też typ, rozmiar, liczbę dowiązań i brak zapisu dla grupy oraz
pozostałych użytkowników. Porównuje `(device, inode, size, mtime_ns,
ctime_ns)` przed odczytem, po odczycie, pod nazwą w chroot oraz w nadal
otwartym deskryptorze VMM. Jeżeli VMM zniknie, FD się zmieni, inode zostanie
podmieniony albo bajty nie zgadzają się z manifestem, odbiór jest odrzucany.

Test kontraktowy obejmuje prawidłowy deskryptor, zły digest, brak FD,
podmianę nazwy na nowy inode o identycznych bajtach po otwarciu przez VMM
oraz mutację tego samego inode podczas haszowania. Wszystkie 17 testów
`scripts.tests.test_stage039_live_firecracker` przeszło lokalnie. Testy
syntetyczne nie zastępują ponownego pełnego przebiegu KVM w CI na commicie
z tą zmianą.

## Granice przeglądu

- Host posiadający uprawnienia root nadal może modyfikować dowolny obraz;
  ten dowód wykrywa zmianę w obserwowanym oknie, lecz nie gwarantuje
  przyszłej niezmienności ani odporności na złośliwe jądro hosta.
- Pozostałe obrazy i binaria wymagają osobnego powiązania z faktycznie
  używanymi deskryptorami lub procesami; nie wolno rozszerzać tego dowodu
  na wszystkie zasoby VM.
- Wykonano tylko ograniczony negatywny test dostępu do pliku hosta opisany
  niżej. Szerszy test ucieczki z guest, przegląd podatności wszystkich
  zależności i test na dwóch trwałych hostach pozostają otwarte.
- API alertów Dependabot dla tego tokenu zwróciło HTTP 403
  (`Resource not accessible by integration`), więc widoczne osiem alertów
  z GitHub nie zostało przypisanych do pakietów ani zakresu 0.39.

## Dodatkowa próba granicy guest→host (realne CI KVM)

`scripts/stage039_guest_host_canary.py` buduje statyczny PID1, wstrzykuje go
do prywatnej kopii ext4 i uruchamia przez jailer. Losowy plik `host-canary`
zostaje umieszczony w hostowym katalogu chroot VMM, lecz poza blokowym
obrazem rootfs gościa. PID1 montuje proc i próbuje otworzyć ten plik przez
ścieżkę bezpośrednią oraz `/proc/1/root/...`. Raport może powstać wyłącznie
po dokładnym markerze z konsoli gościa, potwierdzeniu realnego boot przez API
i cgroup, sprawdzeniu FD VMM, dwukrotnym hash check pliku hostowego oraz
SIGKILL i opróżnieniu cgroup. Brak markera lub powodzenie `open` kończy
próbę błędem. Workflow `stage039-real-firecracker-jailer` wymaga tej próby.

Lokalnie przeszły trzy testy parsera markera i kompilacja statycznego C z
`-Werror`. [Run 40201198228](https://github.com/mojealterego/ocaf-runtime/actions/runs/40201198228)
na commicie `86a37eb` zakończył się sukcesem po zmianie haszowania FD;
artefakt KVM ID `12602395232`, SHA-256 archiwum
`d6c81648fc8fb05c8cdc9e2b33a1d03bf5b103fbbbd56be69c01ef7228647c29`.
[Run 40201458119](https://github.com/mojealterego/ocaf-runtime/actions/runs/40201458119)
na commicie `d99b239` zakończył się sukcesem z nowym wymaganym krokiem
rzeczywistego guest host-canary. Artefakt ID `12602285733`, SHA-256 archiwum
`9f724d56e2a8172365a1c765f2fd74b988d1d0331ee4b97e541435a54a0fdec4`.
Zielony workflow obejmuje sprawdzenie dokładnego markera gościa, boot/API,
cgroup i niezmienionego hostowego canary. Próba dowodzi wyłącznie odmowy
odczytu konkretnego pliku hostowego dwiema ścieżkami; nie dowodzi
niemożliwości ucieczki z VM. 0.39.10 pozostaje częściowy z powodu
niezamkniętego przeglądu podatności i szerszego zakresu escape.
