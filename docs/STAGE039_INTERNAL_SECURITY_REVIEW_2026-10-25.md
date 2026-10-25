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
- Nie wykonano próby ucieczki z guest, przeglądu podatności wszystkich
  zależności ani testu na dwóch trwałych hostach. 0.39.10 pozostaje otwarty.
- API alertów Dependabot dla tego tokenu zwróciło HTTP 403
  (`Resource not accessible by integration`), więc widoczne osiem alertów
  z GitHub nie zostało przypisanych do pakietów ani zakresu 0.39.

## Dodatkowa próba granicy guest→host (oczekuje na CI KVM)

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
`-Werror`. Bieżące środowisko nie ma `/dev/kvm`, więc wykonanie wewnątrz
gościa pozostaje **niepotwierdzone** do zielonego uruchomienia workflow.
Nawet wtedy próba dowodzi wyłącznie odmowy odczytu konkretnego pliku
hostowego dwiema ścieżkami; nie dowodzi niemożliwości ucieczki z VM.
