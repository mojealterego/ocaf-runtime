# 0.41.6 — uwierzytelniona kopia PostgreSQL, zakres laboratorium

`scripts/stage041_encrypted_backup.py` szyfruje rzeczywisty plik `pg_dump -Fc`
za pomocą AES-256-GCM. Metadane generacji, identyfikator klucza i skrót
oryginalnej kopii są związane z szyfrogramem przez AAD. Odtwarzanie sprawdza
tag, skróty, minimalną generację i listę odwołanych kluczy przed zapisaniem
jakichkolwiek bajtów docelowych. Pliki wyjściowe powstają z prawami `0600`.

Workflow `stage041-real-backup-restore.yml` wykonuje lokalny roundtrip na
prawdziwym pliku kopii PostgreSQL. Osobno próbuje zmienić szyfrogram, użyć
innego lub odwołanego klucza oraz obniżyć generację; każda próba musi zostać
odrzucona. Klucz jest losowany w pamięci runnera i nie trafia do artefaktu.

Workflow `stage041-cross-host-restore.yml` przesyła **wyłącznie szyfrogram**
kopii przez artefakt GitHub Actions. Klucz z jednego przebiegu przechodzi
osobno przez output joba; drugi efemeryczny runner Linux odrzuca zmieniony
szyfrogram, błędny lub odwołany klucz i cofniętą generację, odszyfrowuje
kopię i uruchamia rzeczywisty `pg_restore` na innym kernelu. Weryfikacja przed
uploadem odmawia publikacji przy dodatkowym pliku plaintext, brakującym
składniku albo błędnym tagu AES-GCM; negatywne testy działają także z
`python -O`. [Dwukernelowy przebieg 40352308866](https://github.com/mojealterego/ocaf-runtime/actions/runs/40352308866)
potwierdził kontrolę tagu przed uploadem i odtworzenie na drugim kernelu.

To laboratorium nie dowodzi trwałego offsite, niezmienności magazynu,
niezależnego KMS ani awarii fizycznego hosta. Output joba GitHub nie jest
produkcyjnym kanałem dystrybucji klucza. `stage_041_complete=false` i
`production_approved=false` pozostają wymagane. Do odbioru 0.41.6 nadal
potrzebne są niezależny magazyn z retencją WORM, trwały KMS, polityka
rotacji/odwołania i ćwiczenie odtworzeniowe na trwałym obcym hoście.
