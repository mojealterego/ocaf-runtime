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
