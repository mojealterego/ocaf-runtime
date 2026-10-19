# OCAF — obowiązkowy kontrakt implementacji agentów

Każdy Agent, System Agentowy, Rój, narzędzie OMAt i serwer OMAs musi mieć **własny folder**, rzeczywistą implementację **Rust** w pakiecie Cargo, uruchamialne testy jednostkowe, negatywne i integracyjne oraz komplet dokumentacji: TECHNICAL.md, USER_GUIDE.md, CERTIFICATION.md. Transport Zenoh musi być rzeczywiście zaimplementowany, jeżeli wymaga tego architektura. Same pliki Markdown, prompt, skill lub scenariusz Make nie są implementacją.

Maszynowy rejestr component_registry.tsv obejmuje 33 docelowe katalogi rojów, 132 katalogi ich agentów oraz OMAt i OMAs. Status PLANNED oznacza **brak pełnoprawnej implementacji Rust**, nawet jeśli odpowiadający scenariusz Make jest aktywny. IMPLEMENTED_UNVERIFIED oznacza pliki wykonywalne, testy i dokumenty, ale nie potwierdza testów CI. VERIFIED wymaga rzeczywistych dowodów związanych z SHA i odbiorem operatora; same deklaracje w Markdown nie wystarczają.

Przykładowa obowiązkowa struktura folderu:
- Cargo.toml
- src/lib.rs (lub src/main.rs), inne rzeczywiste pliki Rust
- tests/contract.rs oraz dodatkowe testy specyficzne dla architektury
- docs/TECHNICAL.md, docs/USER_GUIDE.md, docs/CERTIFICATION.md
- Zenoh API oraz testy połączeń tam, gdzie transport jest wymagany.

Lokalny walidator OMAt: cargo test --manifest-path omat/contract_guard/Cargo.toml --all-targets, następnie cargo run --manifest-path omat/contract_guard/Cargo.toml -- --repo-root . --registry docs/agents/component_registry.tsv.

Kryterium ukończenia etapu obejmuje wszystkie podetapy, wszystkie implementacje, kompilację, testy pozytywne i negatywne, integrację, rzeczywiste CI, niezależny przegląd, dowody docelowej infrastruktury tam, gdzie wymagana, i formalny odbiór operatora. Walidacja strukturalna nie poświadcza niczego ponad tę strukturę.
