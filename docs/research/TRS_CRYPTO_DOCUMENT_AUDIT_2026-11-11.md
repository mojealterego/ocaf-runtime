# TRS-SEAL: ocena dwóch dostarczonych dokumentów kryptograficznych (2026-11-11)

Status: **analiza materiału wejściowego; brak zatwierdzenia kryptograficznego**.

## Źródła
1. *Zunifikowana Architektura Weryfikacji Topologicznej...* (18 stron): Braid Group, Ko–Lee, Jones, white-box C++.
2. *Przełom kryptograficzny: W pełni post-kwantowa i zaprzeczalna wymiana kluczy* (15 stron): Split-KEM, SPQR, ORAM, RPMB, Android, traffic shaping, DTN.

## Krytyczne ustalenia
| Priorytet | Twierdzenie / implementacja | Ocena | Działanie |
|---|---|---|---|
| P0 | C++ `new int[elements+padding]` zwraca bazę, kod przechowuje przesunięty wskaźnik i wykonuje `delete` zamiast `delete[]` na tym wskaźniku | Undefined behavior; możliwe uszkodzenie sterty | Nie używać implementacji |
| P0 | `CheckTriviality` sprawdza jedynie swobodne kasowanie sąsiednich odwrotności | Nie rozwiązuje problemu słowa grupy warkoczy; fałszywe negatywy dla relacji Artina | Zastąpić poprawnym algorytmem, osobno testować relacje Artina |
| P0 | `TopologicalJonesValidator` XORuje `abs(generator)` | Nie jest wielomianem Jonesa, nie jest MAC ani podpisem; XOR anuluje powtórzenia i gubi znaki | Wykluczyć z uwierzytelniania |
| P0 | Klient ma stały sekret `u={150,151}`; nie ma pełnej wymiany uwierzytelnionej, KDF ani dowodu posiadania klucza | Demonstracja nie realizuje bezpiecznego AKE; podatność na MITM | Używać standardowego uwierzytelnionego KEM / podpisów i KDF |
| P0 | Kotlin `generateSplitKemState()` zwraca 1024 bajty zer, `writeToEnclave` jest puste | Kod nie generuje ani nie przechowuje kluczy | Nie wdrażać jako protokołu |
| P1 | Android JNI zwraca boolean z alokacji, ale dalsza metoda przyjmuje dowolny adres i długość | Brak uchwytu/ownership, walidacji długości i cyklu życia; ryzyko arbitralnego zapisu/zwolnienia | Zastosować opaque handles, RAII, limity, idempotentne zamknięcie |
| P1 | `emergencyAnnihilation()` wywołana pod `Mutex.withLock` i sama próbuje przejąć ten sam mutex | Potencjalny deadlock przy błędzie inicjalizacji | Rozdzielić funkcję wewnętrzną pod blokadą i zewnętrzną |
| P1 | `Random.Default` dla pakietów chaff | Brak gwarancji kryptograficznego CSPRNG | Używać CSPRNG; pakiety chaff generować w poprawnym formacie uwierzytelnionego transportu |
| P1 | `Channel.UNLIMITED` + ciągłe pakiety 16 KiB | Nieograniczony wzrost pamięci przy przeciążeniu; brak backpressure | Kolejka ograniczona, drop policy, telemetry |
| P1 | „constant-rate Poisson” | Proces Poissona ma losowe odstępy, a nie stały interwał; nie daje automatycznie anonimowości względem globalnego obserwatora | Sprecyzować model przeciwnika i metryki wycieku |
| P1 | n=256 i długość słów >=2048 dowodzą bezpieczeństwa Ko–Lee | Brak redukcji do przyjętego problemu trudnego i oszacowania poziomu bezpieczeństwa; znane rodziny ataków algebraicznych | Eksperyment badawczy, bez etykiety PQ-secure |
| P1 | Trudność ewaluacji Jonesa dowodzi odporności na kolizje i MITM | Nie wynika z trudności obliczeniowej; niezmiennik nie jest uwierzytelnionym zobowiązaniem | MAC / podpisy / transcript binding |
| P1 | Ricci neck-pinch, liczby Bettiego i złoty podział dowodzą wykładniczego kosztu LBA | Nie wykazano związku z kosztami algorytmów ani dolnego ograniczenia złożoności | Traktować jako hipotezę badawczą |
| P1 | RPMB, ORAM, mlock, zeroization zapewniają całkowitą odporność na forensics | Zależność od sprzętu, uprawnień, TEE, swap, kopii i modelu atakującego; `mlock` nie jest ORAM | Testy platformowe, opis ograniczeń |
| P1 | „RingXKEM Falcon/MAYO” i Split-KEM są gotowymi standardami do produkcji | Brak kompletnego opisu algorytmów, parametrów, referencji, wektorów testowych i analizy protokołu | Wymagane źródła pierwotne i audyt |
| P2 | Opaque predicates / CFF gwarantują niemożność dekompilacji | Obfuscation podnosi koszt analizy, ale nie daje gwarancji tajności | Defense in depth, nie granica zaufania |

## Bezpieczna ścieżka implementacji
1. **Production:** utrzymać TRS-SEAL jako uwierzytelnioną kopertę HMAC-SHA-256 z izolacją tenantów, fencing i trwałym anti-replay. Zweryfikować kompilację i testy przed wdrożeniem.
2. **PQC:** osobny protokół z ML-KEM (FIPS 203) i odpowiednią autentykacją, transcript binding, KDF, rotacją, downgrade protection; jawny threat model i test vectors.
3. **Braid research:** odrębny moduł eksperymentalny bez dostępu do kluczy produkcyjnych; poprawne operacje Artina, testy relacji, benchmark ataków CSP/LBA i metryki.
4. **White-box:** wyłącznie dodatkowe utrudnienie; preferować hardware-backed key storage i minimalizację sekretów na kliencie.
5. **Transport:** ograniczone kolejki, CSPRNG, metryki opóźnień, budżet energii/bandwidth, testy korelacji; bez deklaracji pełnej niewykrywalności.
6. **Android:** jawne uchwyty natywne, zeroization best effort, bezpieczny lifecycle, bez automatycznego nieodwracalnego kasowania na podstawie niepewnych sensorów.

## Bramka bezpieczeństwa
Żaden element badawczy nie trafia do uwierzytelniania ani derivation kluczy bez: (a) specyfikacji, (b) modelu przeciwnika, (c) poprawnej implementacji i test vectors, (d) niezależnego przeglądu kryptograficznego, (e) testów integracyjnych i regresyjnych.
