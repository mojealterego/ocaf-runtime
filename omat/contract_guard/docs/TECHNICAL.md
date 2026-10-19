# OMAt Contract Guard — dokumentacja techniczna

Status: IMPLEMENTED_UNVERIFIED. To lokalne narzędzie kontrolne, bez komunikacji sieciowej; jego transport to none, nie Zenoh. Nie zastępuje produkcyjnego serwera OMAs.

src/lib.rs parsuje TSV, sprawdza unikalne ścieżki, brak traversal, brak duplikatów i wszystkie katalogi Cargo pod agents, swarms, systems, omat i omas. Dla IMPLEMENTED_UNVERIFIED lub VERIFIED wymaga Rust Cargo.toml, prawdziwego src/*.rs bez todo! i unimplemented!, testów Rust z atrybutami testowymi i trzech dokumentów. Dla transport=zenoh wymaga zadeklarowanej zależności i faktycznego wykorzystania API Zenoh. Dla VERIFIED wymaga pól dowodowych.

PLANNED oznacza niezaimplementowany komponent i nie pozwala twierdzić, że Make prompt lub dokumentacja zastępuje wykonywalne źródła. Testy strukturalne nie poświadczają kompilacji ani zabezpieczeń całego systemu. Walidacja produkcyjna musi dodatkowo obejmować pełne rzeczywiste CI, dowody wielohostowe i decyzję operatora.
