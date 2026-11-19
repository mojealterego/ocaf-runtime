# OCAF Agent Market i ochrona kryptograficzna — kontrakt obowiązkowy v1

**OCAF-AB-MARKET-CRYPTO-V1 · 2026-11-19 · SPECIFIED / OPEN_UNVERIFIED**

Decyzja operatora: każda z ośmiu aplikacji Atelier Sovereign Luxury Professional Edition ma Agent Market, dossier, certyfikaty i wyceny każdego agenta, zakup i wynajem agentów oraz wtyczek; każdy produkt i komponent chronić kryptografią topologiczną i postkwantową. Rozszerzenie AB-OM-14–20 istniejących etapów 0.43, 0.45, 0.49, 0.50 i 0.55, bez renumeracji. Nie jest dowodem implementacji.

## Granice architektury

Wspólny katalog OMAs, registry narzędzi OMAt, OM Engine, istniejące ekonomiczne etapy 0.49/0.87 oraz wspólna polityka bezpieczeństwa — nie osiem niezależnych marketplace. Meta Agent Architekt generuje i weryfikuje kandydatów, wyceny i aktualizacje ofert, Zarządca egzekwuje lifecycle, licencje, zasoby i status. Rozdzielić Product/Offer/Valuation, Artifact/Certificate, Entitlement, Invoice/Settlement i ReleaseApproval. Nie przypisywać samej decyzji Architekta mocy księgowania lub nadawania sobie uprawnień.

- Dossier publiczne lub autoryzowane: dokumentacja, certyfikat jego rzeczywistego zakresu i wydawcy, specyfikacja i kompatybilność, wersja, ceny, warunki licencji, wyniki i historia. Chronione źródła, sekrety, pamięć i dane klientów nie należą do publicznego dossier. DRAFT, NOT_ISSUED, NOT_PRICED i brak backendu są widoczne, nie zastępowane atrapami. Wydanie rynkowe wymaga zweryfikowanego artefaktu, podpisu, ważnej oferty i aktualnej polityki.
- Oferta kupna lub najmu nie sprzedaje własności intelektualnej domyślnie; oznacza licencję na określony zakres. Kwoty w jednostkach minimalnych z walutą, jednoznacznym okresem i kosztami, immutable quote digest przed zgodą. Płatność i uprawnienie są rozdzielone; nie wydawać licencji na podstawie deklaracji klienta. Atomic ledger/idempotency, lease, revocation, refund i odzyskiwanie po awarii; bez realnych transakcji w deterministycznych testach sandbox. Produkcyjna integracja płatności odrębnie zatwierdzana.
- Chroniony artefakt: podpisany manifest obejmujący artifact digest, wersję, typ komponentu, zależności, profile algorytmów i politykę; szyfrowanie kopertowe z osobnym kluczem treści i uwierzytelnieniem metadanych; transport z zachowanym mTLS i zweryfikowanym profilem hybrydowym. Przypięte zaufane korzenie i broker kluczy (hardware-backed gdzie możliwe), rotacja, odwołanie, bounded offline lease, izolacja tenantów i brak automatycznego plaintext export. Gdy dana platforma nie zapewnia wymaganych właściwości, odpowiednia chroniona funkcjonalność fail closed; brak zweryfikowanej ochrony blokuje wydanie pełnego produktu.
- Chronione klasy: wszystkie osiem aplikacji, każdy agent i rój, Meta Agent Architekt, Główny zarządca Agentów, OM Engine, wszystkie OMAs i OMAt oraz OSTc. ML-KEM i ML-DSA uzupełniają audytowane AEAD i istniejące profile TLS/podpisu; konkretne zestawy, rozmiary i parametry w zweryfikowanym ADR. OSTc pozostaje odrębnym, obowiązkowym docelowo mechanizmem topologicznym: bez zatwierdzonej konstrukcji i dowodów nie wolno oznaczać go jako ACTIVE ani zastępować standardowej kryptografii. Brak gwarancji całkowitej antykopiowalności przy kontroli endpointu; szyfrowanie/podpis nie ukrywają już publicznie dostępnego źródła. Chronione własnościowe payloady nie trafiają do publicznego repozytorium jako plaintext; użytkownik zachowuje prawa do własnych projektów i danych. Nazwa OSTc wskazana przez operatora jako chroniona integracja, nie utożsamiać z niepotwierdzonym OCTs.

## Wymagania odbiorowe

### AB-OM-14 · etap 0.50 — Dossier każdego agenta i roju

Karta każdego wpisu (także DRAFT/STAGED) ma trwałe ID, wersję, producenta, pełną dokumentację użytkową/techniczną/architektoniczną/API, ograniczenia, wymagania zasobów, kompatybilność, historię wersji, wyniki testów, rzeczywiste certyfikaty albo jawny status NOT_ISSUED i wycenę albo jawny status NOT_PRICED. Certyfikaty i dokumentacja są przeglądane w aplikacji bez ujawnienia chronionego kodu. Weryfikacja podpisu, digestu, zakresu, ważności i unieważnienia. Fałszywe świadectwo lub niezweryfikowana cena nie mogą być prezentowane jako potwierdzone.

Status: OPEN_UNVERIFIED.

### AB-OM-15 · etap 0.49 — Agent Market — zakup i wynajem

W każdej z 8 aplikacji dostępny jest wspólny tenant-scoped marketplace agentów, rojów oraz zgodnych wtyczek narzędziowych OMAs/OMAt i chronionej integracji OSTc. Zakup licencji i wynajem na określony czas; waluta, jednostka, okres, zakres, koszty wykonania, warunki odnowienia i pełna cena przed zgodą. Rzeczywista autoryzacja przez operatora/użytkownika, transakcyjny ledger, idempotencja, potwierdzenie rozliczenia przed przyznaniem uprawnień, zwroty, cofnięcie i wygaśnięcie. Żadnych automatycznych obciążeń bez zgody; testy sandbox rozliczeń nie są produkcyjnym settlement.

Status: OPEN_UNVERIFIED.

### AB-OM-16 · etap 0.49 — Wycena i oferta wersjonowana

Architekt przygotowuje opartą na porównywalnych benchmarkach, kosztach, popycie i ryzyku wycenę zakupu/wynajmu i wtyczek, z datą, walutą, modelem cenowym, metodą, źródłami i niepewnością. Wycena jest propozycją, nie przychodem; publikowana oferta jest zatwierdzona, wersjonowana, ważna czasowo i przypięta do konkretnego artefaktu/licencji. Nie wymyślać cen, rozliczeń ani zysków.

Status: OPEN_UNVERIFIED.

### AB-OM-17 · etap 0.55 — Chronione licencje i dystrybucja

Podpisane zaszyfrowane pakiety własnościowe i wersjonowane entitlement, powiązane z użytkownikiem/tenantem, zakresem i okresem licencji; klucze przez zaufany broker/chronione magazyny platformy. Zakaz dostępu do plaintext bez uprawnień, nieautoryzowanego klonowania uruchamialnej instancji, replay i downgrade; ograniczony jawny offline lease, bezpieczne odnowienie, unieważnienie i recovery. Użytkownik zachowuje możliwość edycji i eksportu własnego kodu/danych; chroniony kod dostawcy nie jest ujawniany w Code, logach, debug, eksportach ani dossier. Nie obiecywać bezwzględnej odporności na kontrolującego hosta/root/debuggera; publiczne źródła już ujawnione nie mogą stać się tajne przez szyfrowanie kolejnej dystrybucji.

Status: OPEN_UNVERIFIED.

### AB-OM-18 · etap 0.45 — Obowiązkowa postkwantowa kryptografia całego ekosystemu

Wszystkie aplikacje, wszystkie agenty/roje, OM Engine, Architekt, Zarządca, OMAs, OMAt, OSTc, ich pakiety, komunikacja, pamięć i aktualizacje obejmuje standardowy hybrydowy profil kryptograficzny z ML-KEM do ustanawiania kluczy, ML-DSA do podpisów i audytowanym AEAD do danych. Zachować obecne mTLS i podpisy do potwierdzonej interoperacyjności; algorytm i zestaw kluczy związane z kontekstem, tenantem i wersją; downgrade i błędy fail closed. Testy wektorów, interoperacyjności, manipulacji, replay, unieważnienia, rotacji, integralności i pomiar platformowy. Nie zastępować dojrzałej ochrony niesprawdzoną kryptografią.

Status: OPEN_UNVERIFIED.

### AB-OM-19 · etap 0.43 — OSTc — topologiczna ochrona wszystkich komponentów

Obowiązkowe docelowe zabezpieczenie topologiczne wszystkich aplikacji, każdego agenta/roju, OM Engine, Architekta, Zarządcy, OMAs, OMAt oraz samego OSTc. OSTc ma odrębną specyfikację modelu zagrożeń, konstrukcji, parametrów, własności, interfejsu, integracji z PQC i proof/test vectors; nie uznawać niezweryfikowanego prymitywu za bezpieczny. Dotychczasowy OSTc DISABLED i fail-closed pozostają do czasu implementacji, walidacji i zgody; nie omijać blokady źródeł ani podpisów, brak jawnych kluczy w pakietach. Ochrona własnego bootstrappingu OSTc musi wynikać z niezależnego zaufanego root of trust, bez cyklicznej autoryzacji. Dotychczasowa decyzja odłożenia wdrożenia do ostatniego etapu nie oznacza zgody na wydanie produkcyjne bez ochrony.

Status: OPEN_UNVERIFIED.

### AB-OM-20 · etap 0.55 — Odbiór Agent Market i ochrony na wszystkich platformach

E2E każdej z 8 aplikacji w zakresie dossier/certyfikaty/wycena, zakup/wynajem w środowisku testowym, instalacja wtyczki, dopuszczenie licencji i wykonanie przez OM Engine; odmowa obcego tenanta, zmienionego pakietu, wygasłej licencji, nieautoryzowanego kopiowania uruchamialnej instancji, wycieku sekretów, braku OSTc/PQC i downgrade. Dowody per artefakt, platforma, wersja, digest, realne testy albo jawnie ograniczona deterministyczna symulacja niedostępnej infrastruktury. Żadne wcześniejsze scoped E2E ani sam manifest nie zamykają pełnego produktu.

Status: OPEN_UNVERIFIED.


## Kolejność i dowody

Wspólne domeny i rzeczywiste integracje przed odbiorem poszczególnych aplikacji; każda aplikacja po kolei do pełnej bramki, bez deklarowania ukończenia na podstawie samego edytora. OSTc może pozostać końcową pracą implementacyjną według wcześniejszej kolejności, ale żadna chroniona dystrybucja nie uzyskuje produkcyjnego GO przed walidacją OSTc i PQC. Historyczne dowody Linux/Kali zachować jako scoped editor E2E. GATE-10 v2 nadal sprawdza tylko wcześniejszy kontrakt dowodów: wymaga rozszerzenia o AB-OM-01–20; ten dokument nie zmienia wykonywalnego weryfikatora. Zewnętrzny audyt nie jest wymogiem wewnętrznego odbioru; testy deterministyczne muszą być opisane zgodnie z zakresem, nie jako fizyczne wdrożenie. Nie uruchomiono płatności, nie wydano kluczy/certyfikatów, nie wykonano wdrożenia OSTc/PQC.
