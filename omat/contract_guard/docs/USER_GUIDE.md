# OMAt Contract Guard — instrukcja użytkowa

Uruchom od katalogu głównego repozytorium:

cargo test --manifest-path omat/contract_guard/Cargo.toml --all-targets --offline

cargo run --manifest-path omat/contract_guard/Cargo.toml --offline -- --repo-root . --registry docs/agents/component_registry.tsv

Program kończy się kodem zero, jeżeli zgłoszone pakiety mają poprawną wymaganą strukturę, a wszystkie oczekujące wpisy są jawnie oznaczone PLANNED. Dodatni wynik nie oznacza, że wszystkie 33 roje działają w Rust — brakujący katalog musi pozostać PLANNED. Kod błędu 1 oznacza naruszenie kontraktu, a 2 błędne argumenty. Dowody CI i odbiór operatora zawsze wymagają osobnego sprawdzenia.
