# Stan dokumentu: wymagania PDF 1.15–1.18

Źródło: `docs/sources/drive-ocaf/roadmap-115-154.txt`.
Implementacja: `src/document_state.rs`, eksport biblioteki `document_state`.
Numeracja dotyczy PDF, nie kanonicznych punktów Zenoh/CRDT.

- 1.15: `LabelPath` z korzeniem 0, dodatnimi tagami u32, bez aliasów typu 0:01 i bez sierot. Rodzic musi istnieć.
- 1.16: jedna wartość na GUID w etykiecie; kolizja odrzucana także dla zapomnianego atrybutu.
- 1.17: Forget/Remember zachowuje bajty. Publiczne odczyty ukrywają zapomniane atrybuty. Tożsamość logiczna pozostaje, fizyczny adres pamięci nie jest gwarantowany.
- 1.18: nazwana atomowa transakcja, rollback błędnego batcha, Undo/Redo i usuwanie gałęzi redo po nowym commit.

Limity: głębokość 32, 1024 etykiety, 64 atrybuty na etykietę, 4 KiB na payload, 1 MiB payloadów na stan, 8 wpisów historii. Przekroczenie historii odrzuca commit; aplikacja może jawnie wyczyścić historię. Snapshoty są kopiowane, więc nie jest to mechanizm zero-copy. Obiekt jest lokalny: aplikacja musi osobno wiązać dokument z tenantem i uprawnieniami.

Testy biblioteki: 3 PASS (ścieżki i aliasy, atomowość/kolizje/lifecycle/branching, limity i brak rodzica). Komenda: `cargo test --locked --no-default-features --lib document_state::tests`.

Następne zadania: 1.19 trwały kontrakt zdarzeń z SHA-256; 1.20 deterministyczny replay; 1.21 external links; 1.22 persistence. Warstwa nie jest jeszcze podłączona do OMAt ani buildera. Binarki wydaniowe i pakowanie odłożone zgodnie z poleceniem użytkownika.

## Przyrost 1.19–1.22 (2026-10-30)

`src/document_store.rs` dodaje wersjonowany dziennik SQLite oraz interfejs `DocumentDriver` (load/append). Każdy event wiąże tenant, dokument, rewizję, poprzedni SHA-256 i payload. Append stosuje transakcję IMMEDIATE, oczekiwaną rewizję/hash i synchronous=FULL. Błędna mutacja nie zapisuje zdarzenia. Replay odbudowuje wyłącznie lokalny stan; nie posiada callbacków wykonujących efekty zewnętrzne. Undo/Redo i czyszczenie historii są również zdarzeniami.

Limity: 4096 zdarzeń, 5 MiB na kodowane zdarzenie, 32 MiB na dziennik dokumentu. Brak automatycznej kompakcji. Baza musi istnieć w prywatnym katalogu operatora; NOFOLLOW chroni ostatni komponent ścieżki, a prawa katalogów i SQLite sidecars pozostają obowiązkiem aplikacji.

Łańcuch hash nie jest podpisem: podmiot z pełnym prawem zapisu bazy może go przeliczyć. Wykrywanie obcięcia końca wymaga przechowywania oczekiwanego Head poza tym dziennikiem. Load bez Head dopuszcza aktualny stan bazy. Pusty dokument jest stanem genesis.

`resolve_link` sprawdza ten sam tenant, obecność etykiet i przypięty Head celu. Nie zapisuje relacji, nie implementuje topologicznych selektorów Open CASCADE i nie zapewnia atomowego snapshotu wielu dokumentów. Klucze dokumentów muszą pochodzić z uwierzytelnionego mapowania aplikacji.

Wynik: 6 testów biblioteki PASS. Nowe przypadki: ponowne otwarcie bazy i Undo/Redo, stale writer, nieudany batch bez nowego eventu, manipulacja digestem, truncation z Head, cross-tenant link, brak etykiety i nieaktualna wersja celu.

Pozostaje: trwały rejestr linków i selektory 1.21; pozostałe role sterowników schematów/atrybutów i migracje z PDF 1.22; integracja z projektami i autoryzacją; testy awarii procesu/zasilania i kompakcja. Nie deklarujemy pełnego ukończenia 1.19–1.22 ani produkcyjnego wdrożenia. Binarki wydaniowe pozostają odłożone.
