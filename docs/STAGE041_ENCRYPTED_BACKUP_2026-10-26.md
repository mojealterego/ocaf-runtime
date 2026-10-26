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

Ten test nie dowodzi trwałego offsite, niezmienności zewnętrznego magazynu,
osobnego kanału dystrybucji kluczy ani odtworzenia na drugim kernelu z
zaszyfrowanego artefaktu. `stage_041_complete=false` i
`production_approved=false` pozostają wymagane. Do odbioru 0.41.6 nadal
potrzebne są niezależny magazyn z retencją WORM, trwały KMS, polityka
rotacji/odwołania oraz ćwiczenie odtworzeniowe z innego hosta.
