# Zenoh plane OCAF ↔ OMAs ↔ OMAt

OCAF publikuje żądania sesji wyłącznie do osobnego serwera OMAs. OMAs wykonuje
admisję tenantową i przekazuje do osobnego serwera narzędzi OMAt wyłącznie
narzędzia z allowlisty. Zenoh przenosi komunikaty, ale nie nadaje uprawnień.
Bezpośrednia ścieżka OCAF→OMAt jest zabroniona.

- Kontrakt maszynowy: `deploy/zenoh-plane-contract.json`.
- Konfiguracja routera: `deploy/zenoh-ocaf-omas-omat.json5`.
- Konfiguracja wymaga TLS/mTLS, wyłącza scouting, kompresję i niejawne
  protokoły. Ścieżki PKI są przykładowe i wymagają instalacji operatora.
- Health wymaga sesji Zenoh, OMAs, OMAt, polityki tenantów i replay store.
- OSTc pozostaje wyłączone; readiness nie może podnieść agenta do `ACTIVE`.

Walidacja kontraktu jest lokalna i deterministyczna. Nie udaje uruchomienia
routera, OMAs ani OMAt; odbiór E2E wymaga osobnych procesów, certyfikatów,
tenantów i dowodu z docelowego hosta.
