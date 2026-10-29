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
