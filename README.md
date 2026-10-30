<!-- OCAF_README_ROADMAP_BEGIN -->
**AKTUALNY STAN (2026-10-16): 334 etapy główne, 64 podetapy techniczne, 10 produktów Agents Builder; razem 408 pozycji. Następny etap: 335. Audyt 606 repozytoriów: planowany, bez dowodów integracji.**


[![UWAGA CZYTAĆ — ZAKAZ SAMOWOLKI](https://img.shields.io/badge/UWAGA%20CZYTA%C4%86-ZAKAZ%20SAMOWOLKI-ff0000?style=for-the-badge&labelColor=8b0000)](ROADMAP.md)

# 🔴 UWAGA CZYTAĆ — ZABRONIONA JEST SAMOWOLKA 🔴

> **BEZWZGLĘDNY ZAKAZ SAMOWOLKI:** Nie wolno samodzielnie przenosić, usuwać, skracać, zastępować ani renumerować historycznych pozycji ROADMAPY; nie wolno zmieniać jej położenia na początku `README.md`, ukrywać braków, dopisywać nieuzgodnionych funkcjonalności ani oznaczać nieprzetestowanych etapów jako ukończonych. Odzyskane wymagania historyczne pozostają pod oryginalnymi identyfikatorami; nowe wymagania dopisuje się po ostatnim numerze, zgodnie z poleceniem właściciela projektu. Każda zmiana wymaga zachowania historii, aktualizacji manifestu, obu roadmap, README i dziennika zmian oraz rzeczywistych dowodów testów. **ZAKAZ SAMOWOLKI dotyczy także automatycznych generatorów i agentów AI.**

![UWAGA CZYTAĆ — ROADMAPA NA GÓRZE REPOZYTORIUM](https://img.shields.io/badge/UWAGA-CZYTA%C4%86%20ROADMAP%C4%98-red?style=for-the-badge&labelColor=8B0000)

<div align="center">
<h1><font color="red">🔴 UWAGA CZYTAĆ 🔴</font></h1>
<h2><font color="red">PEŁNA ROADMAPA OCAF JEST NA SAMEJ GÓRZE REPOZYTORIUM — NIE USUWAĆ, NIE PRZENOSIĆ, NIE SKRACAĆ!</font></h2>
</div>

> **UWAGA CZYTAĆ — ZASADA BEZWZGLĘDNA:** Aktualny manifest v11 rejestruje **334 etapy główne, 64 podetapy techniczne i 10 produktów Agents Builder (408 pozycji)**. Historyczny stan 215/64/10 pozostaje zachowany w archiwach i changelogu. Nie usuwać ani nie renumerować etapów. Status produkcyjny wymaga niezależnych dowodów.

> **ZASADA STAŁEJ ROZBUDOWY ROADMAPY:** Każdy nowy punkt infrastruktury, architektury, systemu, bezpieczeństwa, integracji lub funkcjonalności OCAF będzie dopisywany do Roadmapy jako kolejna pozycja. Istniejące pozycje i ich historia pozostają zachowane; odzyskane wymagania historyczne wracają pod pierwotne identyfikatory. Po każdym etapie aktualizowane są statusy, dowody testów i dziennik zmian.

# OCAF RUNTIME — ROADMAPA CAŁEGO PROJEKTU (408 pozycji)

## Pakiet agenta i rejestry dowodowe

Referencyjny pakiet zgodności z dostarczoną specyfikacją znajduje się w
[`OCAF-AGENT-001`](<OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/>). Jego bramka cyklu
życia odrzuca `ACTIVE`, gdy OSTc i dostęp do źródeł są wyłączone, także przy
sfałszowanych danych gotowości. Powtarzalne cele to `make agent001-test`,
`make agent001-build` i `make agent001-package`.

Rejestry rozdzielające kod, plan i materiały opisowe: [`AGENT_REGISTRY.md`](docs/AGENT_REGISTRY.md),
[`PROJECT_REGISTRY.md`](docs/PROJECT_REGISTRY.md) oraz [`SOURCE_REGISTRY.md`](docs/SOURCE_REGISTRY.md).

Syntetyczny stan dostawy z 33 rojami i 111 osobnymi agentami, 10 kontraktami Builderów i
aktualnymi dowodami: [`DELIVERY_STATUS_2026-10-21.md`](docs/DELIVERY_STATUS_2026-10-21.md).

**Pełny rejestr w ramce poniżej obejmuje aktualny manifest v11: 334 etapy główne, 64 podetapy techniczne i 10 produktów Agents Builder (408 pozycji). Każdy punkt ma własny numer, opis i status. [Dokument kanoniczny](ROADMAP.md) · [Rejestr zmian](docs/roadmap/STAGE_CHANGELOG.md).

## NAWIGACJA — GDZIE SĄ POZOSTAŁE ETAPY?

**ROADMAPA NIE KOŃCZY SIĘ NA 0.42.2 ANI 0.42.4.** Etapy główne **0.00–2.06 oraz 208–215** i **64 techniczne podetapy** znajdują się **w tej samej ramce**. Podetapy umieszczono bezpośrednio pod ich etapami nadrzędnymi (np. 0.42.1–0.42.4 pod 0.42), a po nich są etapy 0.43–2.06 i 208–215. Dalej, w tej samej tabeli, występuje **10 produktów Agents Builder**.

**[OTWÓRZ PEŁNĄ KANONICZNĄ ROADMAPĘ — wszystkie etapy od początku do końca](ROADMAP.md)** · [Manifest identyfikatorów](docs/roadmap/canonical_v11_inventory.json)

**Aktualny licznik manifestu v11:** 334 etapy główne + 64 techniczne + 10 produktów = **408 pozycji**. Ostatni główny etap: **333**. Następny nowy etap: **334**. Tabela poniżej obejmuje wszystkie 408 pozycji. **ZAKAZ SAMOWOLKI:** nie usuwać, nie skracać i nie renumerować.

## Kanoniczny rejestr 408 pozycji

**To jest pierwsza sekcja README.md i pełna lista statusów, nie tylko odsyłacz.** Każdy numer ma status odzwierciedlający oddzielnie wykonanie, historyczne dowody i odbiór produkcyjny. `ETAP UKOŃCZONY` w punktach 0.00–0.38 oznacza zamknięcie etapów zgodnie z dyspozycją właściciela projektu (0.38: zakończony administracyjnie, przejście do 0.39), a nie niezależnie potwierdzoną gotowość produkcyjną; `ZROBIONE — ZAKRES IMPLEMENTACJI` oznacza kod i testy historyczne obecne w repo (NIE pełny odbiór ani produkcję); `ZROBIONE — TEST W OGRANICZONYM ZAKRESIE` oznacza wyłącznie potwierdzoną próbę laboratoryjną.

**Zakres inwentaryzacji:** manifest v11 obejmuje **334 etapy główne**, **64 podetapy techniczne** i **10 pierwotnych produktów Agents Builder Code/NoCode**. Razem **408 pozycji w README**; produkty nie są dodatkowymi numerami etapów. Historyczna migawka 215/64/10 pozostaje w archiwum i dzienniku zmian. **33 repozytoria referencyjne nie są etapami.**

**Ważne:** brak historycznych dokumentów nie oznacza niewykonania etapu; takie pozycje mają status STATUS HISTORYCZNY DO WERYFIKACJI. Prace zakończone administracyjnie, lecz niepotwierdzone technicznie, też mają status DO ZROBIENIA; opis statusu źródłowego wyjaśnia zakres. Nie uznajemy nieprzetestowanej funkcji za ukończoną. Dane aktualnego manifestu v11 obejmują etapy do **333** (kolejny nowy identyfikator: **334**); wszystkie są w tabeli poniżej. Pierwotne, zagubione wymagania wracają pod starym ID.

**Kanoniczny pełny dokument:** [ROADMAP.md](ROADMAP.md) · [lustrzana Roadmapa](docs/ROADMAP_FULL_ECOSYSTEM_2026.md) · [manifest 333+64](docs/roadmap/canonical_v11_inventory.json) · [dziesięć Agents Builder](docs/roadmap/agents_builder_products_v11.json) · [rejestr odzyskanych braków](docs/INVENTORY_LOST_REQUIREMENTS_2026-10-15.md) · [dziennik aktualizacji po każdym etapie](docs/roadmap/STAGE_CHANGELOG.md).

## Pełna Roadmapa — jedna ramka, wszystkie punkty

| Etap / numer | Opis | Status |
| :-- | :-- | :-- |
| **0.00** | **Historyczny etap bez zachowanej definicji**. Brak opisu w dostępnym README i Roadmapie 0.36–0.99; nie dopowiadać specyfikacji. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `UNSPECIFIED_IN_CURRENT_REPO`. | **ETAP UKOŃCZONY** |
| **0.01** | **Historyczny etap bez zachowanej definicji**. Brak opisu w dostępnym README i Roadmapie 0.36–0.99; nie dopowiadać specyfikacji. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `UNSPECIFIED_IN_CURRENT_REPO`. | **ETAP UKOŃCZONY** |
| **0.02** | **Historyczny etap bez zachowanej definicji**. Brak opisu w dostępnym README i Roadmapie 0.36–0.99; nie dopowiadać specyfikacji. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `UNSPECIFIED_IN_CURRENT_REPO`. | **ETAP UKOŃCZONY** |
| **0.03** | **Historyczny etap bez zachowanej definicji**. Brak opisu w dostępnym README i Roadmapie 0.36–0.99; nie dopowiadać specyfikacji. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `UNSPECIFIED_IN_CURRENT_REPO`. | **ETAP UKOŃCZONY** |
| **0.04** | **Historyczny etap bez zachowanej definicji**. Brak opisu w dostępnym README i Roadmapie 0.36–0.99; nie dopowiadać specyfikacji. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `UNSPECIFIED_IN_CURRENT_REPO`. | **ETAP UKOŃCZONY** |
| **0.05** | **Historyczny etap bez zachowanej definicji**. Brak opisu w dostępnym README i Roadmapie 0.36–0.99; nie dopowiadać specyfikacji. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `UNSPECIFIED_IN_CURRENT_REPO`. | **ETAP UKOŃCZONY** |
| **0.06** | **Durable local jobs (v0.6)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Brak opisu w dostępnym README i Roadmapie 0.36–0.99; nie dopowiadać specyfikacji. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `UNSPECIFIED_IN_CURRENT_REPO`. | **ETAP UKOŃCZONY** |
| **0.07** | **Authenticated capability coordinator (v0.7)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Brak opisu w dostępnym README i Roadmapie 0.36–0.99; nie dopowiadać specyfikacji. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `UNSPECIFIED_IN_CURRENT_REPO`. | **ETAP UKOŃCZONY** |
| **0.08** | **Distributed job orchestration (v0.8)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Brak opisu w dostępnym README i Roadmapie 0.36–0.99; nie dopowiadać specyfikacji. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `UNSPECIFIED_IN_CURRENT_REPO`. | **ETAP UKOŃCZONY** |
| **0.09** | **Local active/passive coordinator and transport preflight (v0.9)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Brak opisu w dostępnym README i Roadmapie 0.36–0.99; nie dopowiadać specyfikacji. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `UNSPECIFIED_IN_CURRENT_REPO`. | **ETAP UKOŃCZONY** |
| **0.10** | **Historyczny etap bez zachowanej definicji**. Brak opisu w dostępnym README i Roadmapie 0.36–0.99; nie dopowiadać specyfikacji. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `UNSPECIFIED_IN_CURRENT_REPO`. | **ETAP UKOŃCZONY** |
| **0.11** | **PostgreSQL active/passive coordinator (v0.11)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.12** | **Transactional audit and crash fault injection (v0.12)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.13** | **Cluster observability and operator controls (v0.13)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.14** | **Stable-endpoint reconnect and deployment hardening (v0.14)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.15** | **Cryptographic Zenoh mTLS integration gates (v0.15)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.16** | **PostgreSQL-authoritative job lease clock (v0.16)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.17** | **Durable leadership fencing epochs (v0.17)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.18** | **Remote-worker epoch pre-execution checks (v0.18)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.19** | **Transactional PostgreSQL effect sink (v0.19)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.20** | **Fail-closed cluster execution & effect privileges (v0.20)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.21** | **Independent authenticated effect receiver (v0.21)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.22** | **Cluster-scoped verified sink permits (v0.22)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.23** | **Automatic PostgreSQL HA failover and namespace isolation (v0.23)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.24** | **Network and infrastructure chaos scenarios (v0.24)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.25** | **Production operations: scoped health and secure configuration (v0.25)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.26** | **Final development acceptance (v0.26)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.27** | **Kontrola gotowości wydania (v0.27)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.28** | **Łańcuch dostaw i potwierdzanie pochodzenia (priorytet 0.28)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.29** | **Ograniczenia ryzyka transportu Zenoh (priorytet 0.29)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.30** | **Migracja zależności Zenoh i audyt RustSec (priorytet 0.30)** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.31** | **Priorytet 0.31 — naprawa artefaktów CycloneDX i atestacji** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.32** | **Priorytet 0.32 — weryfikacja wydania i atestacji** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.33** | **Priorytet 0.33 — testy PostgreSQL na trzech hostach** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.34** | **Priorytet 0.34 — kontrola cyklu życia certyfikatów mTLS** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.35** | **Priorytet 0.35 — kontrolowana wymiana kluczy HMAC agentów** — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md). Historyczny nagłówek README; szczegóły etapu wymagają odtworzenia z historii. **Kryterium zakończenia:** Weryfikacja źródła historycznego i rzeczywistych testów.. **Stan źródłowy:** `HISTORICAL_NEEDS_VERIFICATION`. | **ETAP UKOŃCZONY** |
| **0.36** | **Identity/RPC**. Application-layer certificate private-key proof + HMAC and pinning **Kryterium zakończenia:** Existing CI tests; genuine transport TLS peer binding remains blocked. **Stan źródłowy:** `PARTIALLY_VERIFIED`. | **ETAP UKOŃCZONY** |
| **0.37** | **Identity/RPC**. Separate TLS 1.3-bound ingress verifies live session leaf against RPC sender pin and SAN; CRL/reload/worker migration remains **Kryterium zakończenia:** Real handshake and swapped-cert tests in CI; external PKI/CRL and multi-host still pending. **Stan źródłowy:** `PARTIALLY_VERIFIED`. | **ETAP UKOŃCZONY** |
| **0.38** | **Execution**. Restricted Wasmtime WASIp1 adapter (see [implementation](WASI_EXECUTION_038.md) and [acceptance contract](PRIORITY_038_EXECUTION_CONTRACT.md)) **Kryterium zakończenia:** Operator requested project-level closure and transition to 0.39; documented in [directive](STAGE038_OPERATOR_DIRECTIVE_2026-10-15.md). Passing CI is not persistent two-host acceptance, independent signed security audit or verified production release.. **Stan źródłowy:** `CLOSED_BY_OPERATOR_TECHNICAL_OPEN`. | **ETAP UKOŃCZONY** [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | [PDF11: six-phase Rust/Zenoh roadmap, source obligations; require real integration tests, security negatives and SHA evidence.]
| **0.38.1** | **ADR: wybór przypiętego Wasmtime, model WASIp1, importy i licencje**. Udokumentowany wybór silnika, ograniczeń WASIp1, przypięcie binarium i audyt zależności; implementacja PinnedWasmtime istnieje, niezależny audyt licencji/RustSec niepotwierdzony. **Kryterium zakończenia:** ADR i SHA-pinning silnika oraz niezależnie zweryfikowany audyt licencji/RustSec. **Stan źródłowy:** `PARTIALLY_IMPLEMENTED_ACCEPTANCE_OPEN`. | **CZĘŚCIOWO WDROŻONE — ODBIÓR OTWARTY** |
| **0.38.6** | **Deterministyczny profil wykonania WASIp1**. Zablokowanie lub kontrolowanie czasu, losowości, sieci i niedeterministycznych host calls; powtarzalny wynik w zadeklarowanym profilu. **Kryterium zakończenia:** Dwa uruchomienia identycznego modułu i wejścia na przypiętym silniku dają identyczny wynik; negatywne testy niedozwolonych host calls. **Stan źródłowy:** `UNVERIFIED_ACCEPTANCE_OPEN`. | **WYMAGA WERYFIKACJI — ODBIÓR OTWARTY** |
| **0.38.8** | **Hardening procesu, anulowanie i cleanup WASI**. WasiProcessGroup, SIGKILL on Drop, no_new_privs, RLIMIT_CORE=0, ograniczenia I/O i współbieżności; dowody laboratoryjne opisane w kontrakcie. **Kryterium zakończenia:** Testy przerwań, restartów, potomków, wycieków zasobów i ograniczeń hosta na rzeczywistym silniku; formalny odbiór nadal otwarty. **Stan źródłowy:** `IMPLEMENTED_LAB_EVIDENCE_ACCEPTANCE_OPEN`. | **ZROBIONE — ZAKRES IMPLEMENTACJI; ODBIÓR OTWARTY** |
| **0.38.9** | **Harness CI i negatywne E2E sandboxa WASI**. Testy wasi_sandbox, mutacje parsera, limity I/O, podpisy, anulowanie; dodany test wyczerpania fuel z pozytywną kontrolą, rzeczywisty wynik niepotwierdzony. **Kryterium zakończenia:** Zielone CI na końcowym SHA, rzeczywisty Wasmtime z potwierdzonym fuel trap i testy negatywne; niezależny odbiór niepotwierdzony. **Stan źródłowy:** `IMPLEMENTED_TEST_HARNESS_ACCEPTANCE_OPEN`. | **ZROBIONE — ZAKRES IMPLEMENTACJI; ODBIÓR OTWARTY** |
| **0.39** | **Execution** | Firecracker microVM broker + Aya/eBPF isolation [PDF11: six-phase source; require verifiable E2E and negative tests, not sample code.] [PDF12 s.11-13: E2B Firecracker egress secret broker and supervised sandbox recovery; not yet verified.] [PDF13 s.1-12: Firecracker Jailer cgroups seccomp with Aya/eBPF host-side anomaly detection; verified checkpoint-based fail-closed recovery; host eBPF cannot inspect all guest kernel events; wymagane testy i dowody.] [PDF14 s.5-6,10-12: Code-as-a-Tool sandbox Firecracker; guest KVM, lifecycle, Jailer, cgroups, egress broker and Aya/eBPF host limits; negative tests; źródło nie dowodzi wdrożenia.] [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | **PASS:** [Real Linux KVM CI 38739812121](https://github.com/mojealterego/ocaf-runtime/actions/runs/38739812121): two per-tenant kernel SO_PEERCRED AF_UNIX broker reservations with SHA-pinned netless manifests persisted in SQLite BEFORE launching two simultaneous jailed Firecracker Linux guests; lease/VM id/cgroup/boot hash and SIGKILL/peer survival receipts bound, leases released. [Ledger](STAGE039_040_BROKER_KERNEL_PG_INDEPENDENCE_2026-10-15.md). [Real KVM CI 40051055265](https://github.com/mojealterego/ocaf-runtime/actions/runs/40051055265) proves distinct signed-manifest WASI modules executed inside two jailed guests and real guest `openat` BPF events; signer remains ephemeral, Aya guest ELF, multi-host and operator acceptance remain open; production NO-GO. **FAIL:** Ograniczenia testów i negatywne przypadki zgodnie z dokumentacją źródłową. [PDF01–09: podpisany WASI w gościu, broker egress, SSRF/DNS-rebinding, deny on outage; testy wielohostowe i tenantów] [PDF10 s.5–6: MicroVM lifecycle, teardown, signed guest WASI, TLS-bound egress broker, DNS rebinding/SSRF and fail-closed] Decyzja właściciela 2026-10-26: administracyjne zamknięcie na podstawie [podpisanej symulacji](docs/STAGE039_OPERATOR_DIRECTIVE_2026-10-26.md); nie oznacza produkcyjnego odbioru trwałych hostów ani HA. | **ETAP UKOŃCZONY DECYZJĄ WŁAŚCICIELA — SYMULACJA; TECHNICZNY NO-GO** | [ROADMAP_FULL_ECOSYSTEM_2026.md](ROADMAP_FULL_ECOSYSTEM_2026.md) |
| **0.39.1** | **`stage039_microvm_gate.py`: typed manifest, pinned digests, host preflight**. `stage039_microvm_gate.py`: typed manifest, pinned digests, host preflight **Kryterium zakończenia:** Odrzuca fałszywe /dev/kvm, dane tenant, mutable paths; nigdy nie raportuje boot. **Stan źródłowy:** `**PASS 12 tests**, CI host NO-GO`. | **DO ZROBIENIA** |
| **0.39.2** | **`stage039_broker_ledger.py`: SQLite WAL, fencing leader_epoch, anti-replay**. `stage039_broker_ledger.py`: SQLite WAL, fencing leader_epoch, anti-replay **Kryterium zakończenia:** Odrzuć rollback epoch, concurrent double-claim, błędny owner, DB perms. **Stan źródłowy:** `**PASS 17 tests**`. | **ZROBIONE** |
| **0.39.3** | **`stage039_broker_ipc.py`: AF_UNIX, kernel SO_PEERCRED**. `stage039_broker_ipc.py`: AF_UNIX, kernel SO_PEERCRED **Kryterium zakończenia:** Zła UID i tenant denied; limit ramki, bez implicit exec. **Stan źródłowy:** `**PASS 10 tests**`. | **ZROBIONE** |
| **0.39.4** | **0.39.4 — **KVM kernel ioctl probe** i konfiguracja jails/VM**. **KVM kernel ioctl probe** i konfiguracja jails/VM **Kryterium zakończenia:** Real KVM API_VERSION=12, CREATE_VM succeeded; char device nie wystarcza. **Stan źródłowy:** `**REAL KVM API 12 and CREATE_VM PASS as root, run 38704086975**`. | **ZROBIONE — TEST W OGRANICZONYM ZAKRESIE** |
| **0.39.5** | **Pin Firecracker **i jailer tej samej wersji**, signed/provenance assets; generator staging chroot**. Pin Firecracker **i jailer tej samej wersji**, signed/provenance assets; generator staging chroot **Kryterium zakończenia:** No stale artifact path, immutable verified staged copy, no symlink / TOCTOU. **Stan źródłowy:** `**Static chroot config + 6 tests PASS; live jailer / TOCTOU runtime proof OPEN**`. | **DO ZROBIENIA** |
| **0.39.6** | **Realny boot Firecracker za jailer na host Linux/KVM**. Realny boot Firecracker za jailer na host Linux/KVM **Kryterium zakończenia:** Kernel/rootfs readonly; VM Running + kernel serial + cgroup proof + cleanup. **Stan źródłowy:** `**LIVE EXECUTOR WRITTEN; 6 contract tests PASS; host KVM NO-GO**`. | **DO ZROBIENIA** |
| **0.39.7** | **Program Rust `aya-ebpf` cgroup_skb/egress + userspace Aya loader**. Program Rust `aya-ebpf` cgroup_skb/egress + userspace Aya loader **Kryterium zakończenia:** Realne BPF verifier load, program ID, tenant A block/B unaffected, recovery after detach. **Stan źródłowy:** `**REAL kernel Aya attach and A/B negative+recovery PASS, run 38704086975**`. | **ZROBIONE** |
| **0.39.8** | **Spójny broker lifecycle: claim → jailer → eBPF link → VM → receipt → teardown**. Spójny broker lifecycle: claim → jailer → eBPF link → VM → receipt → teardown **Kryterium zakończenia:** Block until eBPF enforcement; kill/replay/restore cleanup, no orphan VM. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.39.9** | **Dwa odrębne trwałe hosty KVM, tenant A/B, chaos i bounded 100-cycle soak na każdym**. Dwa odrębne trwałe hosty KVM, tenant A/B, chaos i bounded 100-cycle soak na każdym **Kryterium zakończenia:** Rzeczywiste UUID hosta, failure-domain, restart, revoked-key rejection. **Stan źródłowy:** `**NO-GO / hosts unavailable**`. | **DO ZROBIENIA** |
| **0.39.10** | **Safety & supply chain: SBOM, vulnerability/TOCTOU tests, independent scope review**. Safety & supply chain: SBOM, vulnerability/TOCTOU tests, independent scope review **Kryterium zakończenia:** Reproduce SHA manifest-to-running inode, guest escape and egress attempts. **Stan źródłowy:** `**NO-GO**`. | **DO ZROBIENIA** |
| **0.39.11** | **Produkcyjny operator acceptance dla 0.39**. Produkcyjny operator acceptance dla 0.39 **Kryterium zakończenia:** Oddzielne zatwierdzenie, report ze środowiska i podpis; nie mylić z administracyjnym unblock. **Stan źródłowy:** `**NO-GO**`. | **DO ZROBIENIA** |
| **0.39.12** | **Test izolacji jailed guest TAP**. Test izolacji jailed guest TAP **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.39.13** | **Kernel tc/eBPF egzekwowanie polityki**. Kernel tc/eBPF egzekwowanie polityki **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.39.14** | **Handoff dwóch hostów CI**. Handoff dwóch hostów CI **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.39.15** | **TLS1.3 mTLS SAN principal**. TLS1.3 mTLS SAN principal **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.39.16** | **Fencing przy partycji sieci**. Fencing przy partycji sieci **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.39.17** | **Integralność podpisanych dowodów**. Integralność podpisanych dowodów **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.39.18** | **Broker SO_PEERCRED z dwoma jailed VM**. Broker SO_PEERCRED z dwoma jailed VM **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.39.19** | **Podpisać/haszować obraz WASIp1 i digest host↔guest, umieścić **rzeczywisty interpreter WASI wewnątrz rootfs Firecracker**, serial output i sprawdzony exit code.**. Podpisać/haszować obraz WASIp1 i digest host↔guest, umieścić **rzeczywisty interpreter WASI wewnątrz rootfs Firecracker**, serial output i sprawdzony exit code. **Kryterium zakończenia:** W prawdziwym `/dev/kvm`, kernel boot + wykonany wewnątrz gościa import WASI `fd_write` (nie sam host Wasmtime). Artefakt serial `WASI_EXEC:PASS` plus digest modułu i interpreter, test zmienionego modułu FAIL.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.39.20** | **Broker `SO_PEERCRED`/TLS identity→ledger `reserved`→inny `guest_rootfs` digest per tenant→Firecracker→WASI workload receipt→`commit/release`.**. Broker `SO_PEERCRED`/TLS identity→ledger `reserved`→inny `guest_rootfs` digest per tenant→Firecracker→WASI workload receipt→`commit/release`. **Kryterium zakończenia:** Co najmniej dwa równoczesne jailed goście, **różne** WASI input/nonce, brak cross-tenant replay, wymuszone zatrzymanie A nie wpływa na B, brak lease continuation po awarii.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.39.21** | **Revocation, timeout, CPU/memory/time budget, OCI seccomp i real eBPF egress dla WASI.**. Revocation, timeout, CPU/memory/time budget, OCI seccomp i real eBPF egress dla WASI. **Kryterium zakończenia:** CPU/timeout trap, out-of-memory, guest child process SIGKILL, seccomp denies, `tc` guest packet deny and recovery, audit receipts signed by host execution authority.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.39.22** | **Guest ABI eBPF readiness**. Sprawdzić dostępność BTF, kprobe i syscall ABI w faktycznym jądrze guest **Kryterium zakończenia:** Niepusty guest BPF program ID potwierdzony przez Linux guest. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.39.23** | **Podpisany guest probe manifest**. Pakiet kernel/runtime/sonda WASI/guest rootfs ma oddzielne digest i signed provenance **Kryterium zakończenia:** Uruchomienie VM tylko po sprawdzeniu każdego hashu przed mountem. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.39.24** | **Guest BPF quarantine action**. Rzeczywiste wstrzymanie wybranego guest PID przez autoryzowaną politykę i userspace agent **Kryterium zakończenia:** Odczyt guest exit code i brak wykonania innego tenantu. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.39.25** | **BPF joint failure drill**. Host eBPF, guest eBPF i broker fail-closed w rzeczywistym Firecracker **Kryterium zakończenia:** Net partition i crash guest agent dają zweryfikowaną izolację. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.39.26** | **WASI guest provenance**. Bind live WASIp1 execution receipt to broker lease and kernel identity **Kryterium zakończenia:** Two real guest nonces show distinct Wasmtime child exit and hashes. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.39.27** | **In guest eBPF loading**. Build and actually insert Aya syscall probe inside Firecracker kernel **Kryterium zakończenia:** Authenticated guest BPF program ID and openat events collected. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.39.28** | **Guest process quarantine**. Enforce authorized PID isolation after corroborated syscall anomaly **Kryterium zakończenia:** Guest child is stopped while second tenant VM remains alive. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.39.29** | **VM trace propagation**. Carry broker traceparent through Firecracker boot and guest receipt **Kryterium zakończenia:** Collector correlates two independent VM traces under correct tenant. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.40** | **HA**. Three physical host PostgreSQL HA, real leader fencing and failover **Kryterium zakończenia:** [Three independent Linux kernel local failover run 38740936728](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740936728); [30s 8-client WAL soak 38740936947](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740936947); [real Linux veth/netns live PostgreSQL network partition run 38752910618](https://github.com/mojealterego/ocaf-runtime/actions/runs/38752910618) proves remote TCP client IP B/C, a live kernel route blackhole B while C advances, healing, A IMMEDIATE stop, B promotion and C reparent. Netns clients share **ONE kernel**; live *inter-kernel*/physical-host replication, witness fencing, host power-loss still unverified. [Evidence](STAGE040_LIVE_TCP_NETNS_PARTITION_2026-10-15.md).. **Stan źródłowy:** `ACTIVE_TECHNICAL_OPEN`. | **DO ZROBIENIA** |
| **0.40.1** | **Lokalne failover PG na trzech kernelach**. Lokalne failover PG na trzech kernelach **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.40.2** | **pgbench streaming WAL**. pgbench streaming WAL **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.40.3** | **Dwukierunkowy TCP inter-host test PostgreSQL primary na oddzielnym kernelu; standby `primary_conninfo` z replikacją WAL.**. Dwukierunkowy TCP inter-host test PostgreSQL primary na oddzielnym kernelu; standby `primary_conninfo` z replikacją WAL. **Kryterium zakończenia:** LSN WAL na fizycznie **dwóch oddzielnych kernels**: host A writes; B `pg_stat_wal_receiver.status=streaming`; B reads same checksum while A actually sends via network. GitHub artifact-mediated transfer **nie jest** TCP.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.40.4** | **Trzeci host replica i external quorum lease (PostgreSQL cannot be own fencing authority).**. Trzeci host replica i external quorum lease (PostgreSQL cannot be own fencing authority). **Kryterium zakończenia:** 3 odrębne kernels; stale primary blocked from effect-writes; split-brain/partition and restart fence, bounded RPO/RTO with witness.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.40.5** | **Recovery of promotion C after B failure, PG rewind/slot catch-up, WAL backpressure, synchronous mode.**. Recovery of promotion C after B failure, PG rewind/slot catch-up, WAL backpressure, synchronous mode. **Kryterium zakończenia:** Lag/LSN under noisy neighbor and partition, no lost-ack synchronous commits, reject oldest WAL replay.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.40.6** | **PG TCP przez trzy netns, jeden kernel**. PG TCP przez trzy netns, jeden kernel **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.40.7** | **Transport health instrumentation**. PostgreSQL HA telemetry pg_stat_wal_receiver i lag w każdej niezależnej VM **Kryterium zakończenia:** WAL LSN z dwóch faktycznie niezależnych kernels na live TCP. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.40.8** | **Quorum pre-promotion gate**. Promocja PostgreSQL tylko przy ważnej dzierżawie z trzeciego świadka **Kryterium zakończenia:** Network partition zablokuje stale leader effect writes. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.40.9** | **Postgres fencing soak**. Przy 8-32 klientach i obciętym linku wykonać 60 min wal stress **Kryterium zakończenia:** Porównanie committed tx-id i WAL ACK po failover. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.40.10** | **PQC secured database controls**. Po ustanowieniu prawdziwych peer certów ograniczyć admin/replication privileges **Kryterium zakończenia:** Test TCP mTLS i opcjonalnej hybrydy PQC w niezależnych hostach. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.40.11** | **Live WAL vs artifact recovery distinction**. Require direct two kernel TCP stream to count as distributed HA **Kryterium zakończenia:** Two separate Linux boot IDs continuously exchange actual PostgreSQL WAL. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.40.12** | **Postgres quorum witness**. Require third witness fencing token before standby promotion **Kryterium zakończenia:** Network partition and old primary return never yield two writable nodes. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.40.13** | **Distributed escrow durability**. Persist authorized billing and job reservations on fenced transactional HA store **Kryterium zakończenia:** Failover preserves committed ledger and refuses duplicate settlements. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.40.14** | **WAL failover trace continuity**. Correlate LSN lag and timeline across old and new leader **Kryterium zakończenia:** Real promotion trace shows exact WAL catchup and reconnect. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.41** | **Reliability**. Backup/restore, partition chaos, recovery and prolonged soak **Kryterium zakończenia:** [Physical WAL point-in-time restore on separate kernel run 38738198412](https://github.com/mojealterego/ocaf-runtime/actions/runs/38738198412): primary IMMEDIATE crash, SHA-pinned physical base and WAL tar sent via GitHub artifact, second kernel fresh PG replay to target LSN, 35/35 recovered, 5 later committed rows omitted; compromised bytes and replayed source boot ID rejected. [Detailed evidence](STAGE041_TWO_KERNEL_PHYSICAL_WAL_PITR_2026-10-15.md). Durable physical server power loss, encrypted offsite retention, actual automated RTO/RPO under soak still OPEN.. **Stan źródłowy:** `ACTIVE_TECHNICAL_OPEN`. | **DO ZROBIENIA** |
| **0.41.2** | **PG logic restore na innym kernelu**. PG logic restore na innym kernelu **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.41.3** | **WAL PITR do LSN na jednym kernelu**. WAL PITR do LSN na jednym kernelu **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.41.4** | **Physical PITR na drugim kernelu**. Physical PITR na drugim kernelu **Kryterium zakończenia:** Weryfikacja faktycznych dowodów z dokumentu źródłowego.. **Stan źródłowy:** `HISTORICAL_LAB_SCOPE_LIMITED`. | **DO ZROBIENIA** |
| **0.41.5** | **KVM and postgres 30/60/120 min soak including load ramp, WAL retention, clock skew and injected faults.**. KVM and postgres 30/60/120 min soak including load ramp, WAL retention, clock skew and injected faults. **Kryterium zakończenia:** Crash-only evidence vs whole host fault distinguished; actual throughput p50/p95/p99, RSS, syscall errors, replay lag, post-fault audit.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.41.6** | **Immutable encrypted offsite backup, scheduled restore drill and bounded provenance.**. Immutable encrypted offsite backup, scheduled restore drill and bounded provenance. **Kryterium zakończenia:** Real separate storage/keys; tamper ciphertext / revoked key / rollback deny; restore independent Linux kernel.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.41.7** | **SHM versus network soak**. Mierzyć p50/p95/p99/rate/CPU dla Zenoh SHM i TCP przy obciążeniu **Kryterium zakończenia:** Surowe percentyle i job run ID na tej samej infrastrukturze. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.41.8** | **Telemetry backpressure soak**. OTel kolektor offline i workload running przez 60 min **Kryterium zakończenia:** Zmierzone dropped spans, queue depth, flush i CPU. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.41.9** | **Quarantine false positive soak**. Korpus openat/connect/sendto w gościu i host zdarzenia przez 60 min **Kryterium zakończenia:** Mierzony FPR/FNR, brak cross-tenant event i poprawny cancel. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.41.10** | **Budget and recovery chaos**. Równoległe wyczerpanie token budget + host crash + PostgreSQL PITR **Kryterium zakończenia:** Wymuszone limity i spójne ledger logs po restore. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.41.11** | **Eight step method inventory**. Document per step owner test threat model and source-page traceability **Kryterium zakończenia:** Automation detects missing step for all eight research phases. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.41.12** | **Integrated 60 minute soak**. Stress live broker VM PG WAL and swarms concurrently for at least 60min **Kryterium zakończenia:** Measured duration and p95 memory LSN exits and errors archived. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.41.13** | **Memory and trace chaos**. Fault inject CRDT divergence delayed trace export and Titans restore **Kryterium zakończenia:** No cross tenant leak unbounded queue or revoked snapshot recovery. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.41.14** | **Settlement disaster reconciliation**. Test escrow failover L402 expiry and delayed authorization responses **Kryterium zakończenia:** Post restore ledger matches genuine events without double charge. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.42** | **Release**. Production hardening, SBOM/provenance, license and audit GO/NO-GO **Kryterium zakończenia:** External security review; release version not mistaken for priority number. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** [PDF13 s.1–12: wymagania Rust/Zenoh/CRDT/OTel/L402/ZKP/MCP/Firecracker/PQC/Loihi według audytu; testy i dowody wymagane.] [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | [PDF12 s.11-13: Rig/E2B/Zenoh OTel CRDT/L402 ZKP/MCP fuzzing/PQC Edge AI as applicable; evidence pending.]
| **0.42.1** | **Cargo backwards compatibility**. Stage Rust workspace crates without rewriting existing root runtime binary **Kryterium zakończenia:** All existing cargo tests and CLI entrypoints pass on main. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.42.2** | **Workspace licensing and SBOM**. Record actual upstream license resolved hash and dependency vulnerabilities **Kryterium zakończenia:** Pinned SBOM cargo deny and audit cover all crates. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.42.3** | **Release evidence scope matrix**. Separate real host, guest, Loihi, LN and test lab proof per feature **Kryterium zakończenia:** Production gate demands real evidence IDs and scopes. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.42.4** | **Reproducible rollback**. Sign manifests and test rollback of workspace update on fresh runner **Kryterium zakończenia:** Artifact SHA and prior version behavior recover after rollback. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **0.43** | **OMAs / OMAt** | **KONTRAKT NATYWNY 0.43:** OMAs (OmniAgent Server) i OMAt (OmniAgent Tools) zastępują wewnętrzną architekturę MCP, bez wymaganej zgodności z protokołem MCP. Odrębne rejestry agentów i narzędzi, tożsamości tenantów, autoryzacja, podpisy Ed25519, bezpieczne odkrywanie schematów, natywny transport i dowody E2E; aktualny stan ACTIVE_TECHNICAL_OPEN, NIE produkcyjny GO. **HISTORYCZNE WYMAGANIA MCP (zachowane):** Dynamic discovery, semantic routing, manifest signature, Tree-sitter & JSON Schema [PDF11: six-phase source; require verifiable E2E and negative tests, not sample code.] [PDF12 s.11-13: MCP JSON Schema mutation fuzzing and malicious tool metadata; not yet verified.] [PDF13 s.1-12: MCP tools/list semantic discovery, JSON Schema validation and Tree-sitter/type-sitter CST-preserving fusion; test prompt injection in tool metadata; wymagane testy i dowody.] [PDF14 s.1-3,5,7,11-12: hypothesize-retrieve-invoke, dynamic tools/list, inputSchema/outputSchema, cache expiry, schema integrity, registry trust and rug-pull tests; źródło nie dowodzi wdrożenia.] [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | **PASS:** Untrusted tool metadata cannot inject policy; unknown tool never executes **FAIL:** Ograniczenia testów i negatywne przypadki zgodnie z dokumentacją źródłową. [PDF01–09: signed version-pinned MCP, dynamic schema, JSON Schema, revoke, rug-pull i test 500+ narzędzi] [PDF10 s.2–3: hypothesize–retrieve–invoke, tools/list, inputSchema/outputSchema, context eviction, revocation, ACL, dynamic-vs-static benchmark] | **ACTIVE_TECHNICAL_OPEN** | [ROADMAP_FULL_ECOSYSTEM_2026.md](ROADMAP_FULL_ECOSYSTEM_2026.md) |
| **0.44** | **OAuth** | OAuth 2.1 authorization code + PKCE, scoped tenant tokens, secret broker [PDF11: six-phase source; require verifiable E2E and negative tests, not sample code.] [PDF12 s.11-13: per-user MCP OAuth PKCE endpoint and SSRF-resistant secret broker; not yet verified.] [PDF13 s.1-12: Per-user OAuth PKCE MCP endpoint isolation and hybrid PQC TLS compatibility and downgrade testing; wymagane testy i dowody.] [PDF14 s.3-5,9-10: managed OAuth PKCE, step-up, per-user endpoint, secret broker and confused-deputy isolation; źródło nie dowodzi wdrożenia.] [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | **PASS:** PKCE, CSRF, confused deputy, issuer/audience/replay test matrix **FAIL:** Ograniczenia testów i negatywne przypadki zgodnie z dokumentacją źródłową. [PDF01–09: OAuth PKCE/state per tenant/user, refresh/revoke, confused-deputy, redakcja sekretów] [PDF10 s.3–5: managed OAuth PKCE/state/nonce, per-user endpoint, step-up, refresh/revoke, encrypted secret broker, confused-deputy tests] | **PLANNED_OR_UNVERIFIED** | [ROADMAP_FULL_ECOSYSTEM_2026.md](ROADMAP_FULL_ECOSYSTEM_2026.md) |
| **0.45** | **Crypto**. Hybrid PQC ML-KEM/ML-DSA interoperability, ZKP caveat proof experiments **Kryterium zakończenia:** No replacement of audited mTLS until standards/tests pass. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** | [PDF01–09: ML-KEM/ML-DSA hybrydowe TLS i ZKP z niezależnym verifierem, downgrade i negative witness]
| **0.46** | **Transport** | Zenoh SHM typed zero-copy, CRDT Automerge/Yrs for non-authoritative state [PDF11: six-phase source; require verifiable E2E and negative tests, not sample code.] [PDF12 s.11-13: Zenoh metrics and non-authoritative CRDT without removing fencing; not yet verified.] [PDF13 s.1-12: Zenoh typed SHM zero-copy and backpressure; Automerge/Yrs CRDT delta synchronization with offline partitions only for non-authoritative state; wymagane testy i dowody.] [PDF14 s.11-12: Zenoh pub/sub/queryables, SHM and Automerge CRDT; authoritative fencing/quorum retained; źródło nie dowodzi wdrożenia.] [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | **PASS:** Benchmarks and CRDT convergence; NEVER for leadership/payment authority **FAIL:** Ograniczenia testów i negatywne przypadki zgodnie z dokumentacją źródłową. [PDF01–09: Zenoh SHM tylko lokalnie, Automerge/Yrs convergence; KPI M1 <35us i 10k agentów wyłącznie cele] | **PLANNED_OR_UNVERIFIED** | [ROADMAP_FULL_ECOSYSTEM_2026.md](ROADMAP_FULL_ECOSYSTEM_2026.md) |
| **0.47** | **Telemetry** | W3C tracing propagation, OpenTelemetry/OTLP metrics, SLA and audit logs [PDF11: six-phase source; require verifiable E2E and negative tests, not sample code.] [PDF12 s.11-13: Zenoh router stats OpenTelemetry and trace evidence; not yet verified.] [PDF13 s.1-12: OpenTelemetry OTLP batch exporter, Tokio spawn span instrumentation, parent-based sampling, secret-safe cross-host traces; wymagane testy i dowody.] [PDF14 s.11-12: OpenTelemetry batch exporter, Tokio spans, cross-host trace continuity; źródło nie dowodzi wdrożenia.] [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | **PASS:** No secret leakage; trace replay across actual host boundaries **FAIL:** Ograniczenia testów i negatywne przypadki zgodnie z dokumentacją źródłową. [PDF01–09: M0 500 współbieżnych żądań i p50/p95/p99; testy OTel/OTLP i wyniki SHA] | **PLANNED_OR_UNVERIFIED** | [ROADMAP_FULL_ECOSYSTEM_2026.md](ROADMAP_FULL_ECOSYSTEM_2026.md) |
| **0.48** | **Data**. Enterprise ingest, connectors, retrieval, provenance, bitemporal policy **Kryterium zakończenia:** Tenant isolation, document provenance, deterministic authorization. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.49** | **Economics** | L402/x402 adapter, testnet invoice/preimage verification, offline escrow [PDF11 s.7–9: demonstracyjny preimage i macaroon nie dowodzą płatności; wymagane rzeczywiste settlement, ledger reserve/commit/refund, replay i testy tenantów.] [PDF12 s.11-13: offline L402 caveats, ZKP research and x402 settlement; not yet verified.] [PDF13 s.1-12: L402 Macaroon attenuation and cryptographic preimage verification, replay-safe ledger, bounded offline debt; Arkworks/Halo2 ZKP feasibility; wymagane testy i dowody.] [PDF14 s.6-8,11-12: L402/x402 HTTP402, Macaroon attenuation, cryptographic preimage, tenant budgets, replay protection, offline debt/ZKP R&D; źródło nie dowodzi wdrożenia.] [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | **PASS:** Idempotent ledger, upper spend limits, explicit human confirmation **FAIL:** Ograniczenia testów i negatywne przypadki zgodnie z dokumentacją źródłową. [PDF06 s.7–10: stały preimage i syntetyczny macaroon NIE są płatnością; L402/x402 oddzielnie, settlement, reserve/commit/refund, replay, idempotency] [PDF10 s.7–8: external MCP trust registry, L402/x402 402 challenge, invoice/preimage, macaroon attenuation, budget/idempotency] | **PLANNED_OR_UNVERIFIED** | [ROADMAP_FULL_ECOSYSTEM_2026.md](ROADMAP_FULL_ECOSYSTEM_2026.md) |
| **0.50** | **AGENTS BUILDER — wspólny rdzeń projektowania Code i NoCode**. Pierwotny Agents Builder (NIE generator gotowych agentów): ocaf-builder-core; kanoniczny typowany DAG/IR, kompilacja manifestu projektu agenta, wersjonowanie, galeria szablonów, debugowanie i wspólny backend dla Cloud, Windows, macOS, Android, iOS; dwa oddzielne warianty Code/NoCode na każdej platformie. **Kryterium zakończenia:** Wspólne parsery i typed IR: rzeczywisty projekt agentowy Code i NoCode, zapis/odczyt, kompilacja, roundtrip obsługiwanego podzbioru DSL; 10 prawdziwych buildów produktu i E2E są osobnymi gate 0.52–0.54.. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.51** | **AGENTS BUILDER — silniki edycji Code oraz NoCode**. Pierwotne dwa kompletne tryby budowania: Code — edytor źródeł/DSL, diagnostyka, kompilacja, uruchamianie; NoCode — rzeczywisty interaktywny wizualny canvas drag/drop, paleta node, typed edges, symulator, konfiguracja modeli/Skills/MCP, eksport. Łączy je uczciwa obsługiwana transformacja Code↔NoCode; nie obiecuje dowolnej odwracalności Rust. **Kryterium zakończenia:** Osobne UI i testy użytkownika Code/NoCode; własny edytowalny projekt przechodzi z canvas do DSL i z powrotem w obsługiwanym zakresie, debug real runner i reload projektu.. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.52** | **AGENTS BUILDER CHMUROWY — Code + NoCode (dwa produkty)**. Agents Builder Chmurowy Code: pełne web IDE do projektowania projektów agentowych, typed Code DSL, integracja repo i build/deploy kontrolowany. Agents Builder Chmurowy NoCode: interaktywny browser canvas do budowy/testowania/publikacji agenta bez programowania. Axum multi-tenant backend + RBAC, auth, audyt, stan projektów, limity, eksport. **Kryterium zakończenia:** Dwa osobne Cloud Code i Cloud NoCode browser E2E: tworzenie, edycja, preview, zapis, odtworzenie, kontrolowany build oraz audyt publikacji; negatywne testy izolacji tenantów i zadań kosztowych.. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.53** | **AGENTS BUILDER DESKTOPOWY Windows + macOS — Code i NoCode (cztery produkty)**. Agents Builder Desktopowy Windows Code, Windows NoCode, macOS Code, macOS NoCode — CZTERY odrębne aplikacje/warianty na wspólnym Tauri v2/Rust core. Osobne native Code IDE i interaktywny NoCode canvas na każdej platformie, local-first/offline, debugger, podpisane pakiety, bezpieczny eksport i aktualizacje. Nie traktować macOS jako automatycznie przetestowanego Windows. **Kryterium zakończenia:** Rzeczywiste buildy i uruchomienia dwóch produktów na runnerach Windows ORAZ dwóch na macOS; E2E edycji Code i NoCode, zapisu/offline roundtrip i walidacji signature/entitlements.. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.54** | **AGENTS BUILDER MOBILNY Android + iOS — Code i NoCode (cztery produkty)**. Agents Builder Android Code, Android NoCode, iOS Code, iOS NoCode — CZTERY odrębne warianty aplikacji. Rust UniFFI core, Kotlin Jetpack Compose na Android, SwiftUI na iOS: mobilny edytor Code ograniczonego DSL i prawdziwy dotykowy NoCode canvas, offline, wersje, export, bezpieczny preview. Mobile nie zakłada Firecracker/KVM/JIT i nie obiecuje kompilacji każdej biblioteki on-device. **Kryterium zakończenia:** Android ARM64 emulator lub urządzenie: APK install + test Code oraz UI NoCode; iOS Xcode simulator/device: osobne testy Code/NoCode; import/export, trwałość projektów, resume po ubiciu procesu.. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.55** | **AGENTS BUILDER — security/verification/packaging dla wszystkich 10 wariantów** | Gwarantowany wspólny per-project auth, tenant isolation, typed graph verification, koszt i quota, sandbox preview WASI dla serwerów, ograniczony runner mobilny, podpisany eksport, bezpieczna publikacja, release gate i traceable run receipts — obowiązuje wszystkie pięć platform × Code/NoCode. [PDF11: six-phase source; require verifiable E2E and negative tests, not sample code.] [PDF14 s.5-6: Code-as-a-Tool decorators and typed schema publishing in Agents Builder products, no lossless arbitrary-code conversion claim; źródło nie dowodzi wdrożenia.] [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | **PASS:** Macierz 10/10: real build+GUI E2E każdego wariantu, negative privilege-edge, cycles, cost overrun, podpisań i fake publishing, opublikowany hash artefaktu + stage report. **FAIL:** Jeśli choć jedna z 10 pozycji bez testu rzeczywistego — status całego Agents Builder pozostaje INCOMPLETE; żadnego automatycznego signing/sideload/płatności. [PDF10 s.5–6: Python/TypeScript @tool typed introspection, MCP schema generation, sandbox E2E and unsupported-signature rejection] | **PLANNED_OR_UNVERIFIED** | [odtworzona macierz 10 produktów](AGENTS_BUILDER_CODE_NOCODE_5_PLATFORMS.md) · [pierwotna architektura](MASTER_ARCHITECTURE_26_PDFS.md) |
| **0.56** | **OmniDev**. AST/CST code agents, repo analysis, codegen, test harness **Kryterium zakończenia:** Compilable patches, tests, human-gated protected updates. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** | [PDF01–09: AST/CST patch, rzeczywisty sandbox build/test, security critic, human gate]
| **0.57** | **OmniUX**. Figma design integration, accessible component UI and visual diff **Kryterium zakończenia:** Design token checks, visual regression, accessibility. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.58** | **OmniE2E**. Cross-platform app build and E2E test swarm **Kryterium zakończenia:** Reproducible Android/iOS/Desktop builds and QA artifacts. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.59** | **OmniForge**. Game development/CAD/GPU shader swarm **Kryterium zakończenia:** Sandboxed shader builds, GPU budgets and asset policy. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.60** | **OmniWeb**. Authorized web extraction, workflow automation, site generation **Kryterium zakończenia:** Permissions, rate limits, provenance; no default anti-bot bypass. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.61** | **OmniStudio**. Image/audio/video media orchestration swarm **Kryterium zakończenia:** Model/provider adapters, asset provenance, rights, spending quotas. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.62** | **OmniDoc**. Typst/report/code-aware documentation swarm **Kryterium zakończenia:** Reproducible PDFs and citations; source-map tests. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.63** | **OmniSec** | Purple Team validation in authorized scoped targets [PDF11: six-phase source; require verifiable E2E and negative tests, not sample code.] [PDF12 s.11-13: MCP injection fuzzing and sandbox anomaly tests; not yet verified.] [PDF13 s.1-12: MCP schema mutation fuzzing, indirect injection, sandbox anomaly negative tests and independent evidence; wymagane testy i dowody.] [PDF14 s.8-11: OWASP ASI01-ASI10 including indirect injection, tool misuse, identity, supply-chain rug-pull and RCE; verify each attack separately; źródło nie dowodzi wdrożenia.] [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | **PASS:** Authorization checks, non-destructive default, explicit approval **FAIL:** Ograniczenia testów i negatywne przypadki zgodnie z dokumentacją źródłową. [PDF01–09: OWASP agentic, prompt/tool-output injection, SSRF, sandbox escape i metadata poisoning negative tests] [PDF10 s.9–11: OWASP ASI01–05 explicit negative tests; ASI06–10 only summarized, require independent threat-model detail] | **PLANNED_OR_UNVERIFIED** | [ROADMAP_FULL_ECOSYSTEM_2026.md](ROADMAP_FULL_ECOSYSTEM_2026.md) |
| **0.64** | **OmniGov** | Governance/approval/constitutional policy control plane [PDF12 s.11-13: post-quantum TLS/auth and ZKP feasibility research; not yet verified.] [PDF13 s.1-12: FIPS 203 ML-KEM / FIPS 204 ML-DSA hybrid TLS migration, ZKP and ZKPoU feasibility with interoperability and downgrade resistance; wymagane testy i dowody.] [PDF14 s.12-13: PQC FIPS203/204 hybrid TLS and Arkworks/Halo2 ZKP feasibility, no deployment assumption; źródło nie dowodzi wdrożenia.] [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | **PASS:** Tamper-evident approvals, manual override constraints **FAIL:** Ograniczenia testów i negatywne przypadki zgodnie z dokumentacją źródłową. | **PLANNED_OR_UNVERIFIED** | [ROADMAP_FULL_ECOSYSTEM_2026.md](ROADMAP_FULL_ECOSYSTEM_2026.md) |
| **0.65** | **OmniFin**. FinOps and cost-exhaustion defenses **Kryterium zakończenia:** Per-agent quotas and denied over-budget purchases. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.66** | **OmniPhys**. Robotics and physical-action connector swarm **Kryterium zakończenia:** Simulation-first, e-stop, separate physical permissions. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.67** | **OmniCog + OmniGraph** | Bitemporal memory and provenance-aware knowledge DAG [PDF11: six-phase source; require verifiable E2E and negative tests, not sample code.] [PDF13 s.1-12: Automerge/Yrs CRDT DAG provenance, offline convergence, delta/Bloom synchronization with authoritative fencing retained; wymagane testy i dowody.] [PDF14 s.8,11-12: native PDF/log/code RAG ingestion, async embeddings, provenance, tenant ACL, CRDT nonauthoritative state; źródło nie dowodzi wdrożenia.] [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | **PASS:** Retention, merge lineage, tenant boundaries, stale-memory tests **FAIL:** Ograniczenia testów i negatywne przypadki zgodnie z dokumentacją źródłową. [PDF10 s.8: enterprise RAG recursive PDF/log/code ingestion, provenance, ACL, tenant separation, chunking, reindex/delete] | **PLANNED_OR_UNVERIFIED** | [ROADMAP_FULL_ECOSYSTEM_2026.md](ROADMAP_FULL_ECOSYSTEM_2026.md) |
| **0.68** | **OmniSleep + OmniPrune**. Scheduled consolidation, semantic cache, knowledge decay **Kryterium zakończenia:** Safe pruning with rollbacks, benchmark vs baseline. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.69** | **OmniArbitrage**. Model SLA/BATNA routing, fallback, epsilon exploration **Kryterium zakończenia:** Measured quality/cost/latency under adversarial outages. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.70** | **OmniContract + OmniChaos**. Typed runtime contracts, circuit breakers, load shedding **Kryterium zakończenia:** Bounded retries and negative dependency/chaos tests. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.71** | **OmniStrategy**. DAG planning, counterfactual checks, error memories **Kryterium zakończenia:** Bounded search and independent step outcome verification. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.72** | **OmniSense + OmniEvolve**. Graph planning and governed self-improvement candidates **Kryterium zakończenia:** Shadow-only candidates, no unreviewed promotion to production. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.73** | **OmniVerify + OmniHomeostasis**. SMT-defined invariants and homeostatic resource budgets **Kryterium zakończenia:** Proof scope explicit; arbitrary program termination not provable. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.74** | **OmniCausal**. Counterfactual simulation and immutable deployment approvals **Kryterium zakończenia:** Modelled causal assumptions and no unattended hot-swap. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.75** | **OmniLatent + OmniAdaptive**. Typed embeddings, work stealing and sandboxed ephemeral tools **Kryterium zakończenia:** Model/dimension binding, leased ownership and isolation tests. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.76** | **OmniMath**. Independent Lean 4 proof kernel and selected Z3/cvc5 e-graph cases **Kryterium zakończenia:** Certified selected theorems; LLM consensus not evidence. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.77** | **OmniInference + OmniTDA**. Active inference, epistemic topology and diagnostics **Kryterium zakończenia:** Repeatable metrics with falsifiable hypotheses. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.78** | **OmniHDC + Sparse SAE**. Hyperdimensional computing and sparse feature analysis **Kryterium zakończenia:** Interpretability benchmarks and leakage checks. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.79** | **OmniFractal + OmniNARS**. Hierarchical delegation, uncertainty logic, bounded recursion **Kryterium zakończenia:** No unconstrained self-replication; bounded termination budgets. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.80** | **OmniCategory + OmniLogic**. Category-theoretic mapping, multi-valued logic **Kryterium zakończenia:** Precisely defined type-preserving mappings and theorem checks. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.81** | **OmniTranspile + PCC**. Verified Wasm/Cranelift modules, Hoare pre/postconditions **Kryterium zakończenia:** Independent checker for defined properties; signature not a proof. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.82** | **OmniThermo + OmniQUBO**. Energy cost models, optimization constraints and QUBO experiments **Kryterium zakończenia:** Real power telemetry, optimization reproducibility. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.83** | **OmniAlign**. Human-aligned safety gates, conformal uncertainty, refusals **Kryterium zakończenia:** Adversarial evaluation, audited policy, human approval for risky actions. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.84** | **End-to-end acceptance**. System integration, external review and product release readiness **Kryterium zakończenia:** Signed acceptance for each supported optional domain pack. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.85** | **OmniProof-STARK**. Research a real verifier for formally specified task predicates, not unrestricted model reasoning **Kryterium zakończenia:** Concrete relation, witness/public statement separation, soundness assumptions, valid/invalid/corrupted-proof tests, measured proof sizes/costs. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.86** | **OmniMacro / Causal Emergence**. Experimentally aggregate micro-agent state without losing per-agent audit, permissions or fencing **Kryterium zakończenia:** Baseline, reversible state aggregation, information loss bounds, effect equivalence, falsifiable causal metrics. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.87** | **OmniMarket-L402+**. Optional payment-channel state machine atop 0.49; no direct production settlement authority **Kryterium zakończenia:** Signed sequence/nonces, channel conservation, two-sided dispute and timeout handling, durable replay and manual spending limits. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.88** | **OmniSubstrate**. Sandboxed Wasm/Cranelift specialization of *approved pure* hot paths, optional SIMD, exploratory FPGA **Kryterium zakończenia:** Equivalence tests, independent limits and signatures; performance measured vs baseline, deterministic fallback. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** | [PDF08 s.8–9: ograniczony Rust JIT/hot-swap tylko zatwierdzone pure paths, podpis, rollback]
| **0.89** | **Proof-Carrying RSI**. Candidate code changes with independently checkable Lean/SMT contracts and human-authorized deployment **Kryterium zakończenia:** Real proof kernel, property scope, compiled artifact binding, adversarial invalid proofs; no automated production hot-swap. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.90** | **OmniThermo-Governor**. Hardware-sourced RAPL/power telemetry and budget policy extending 0.82/0.65 **Kryterium zakończenia:** Calibration vs wall power, sensor absence/rollover, per-tenant attribution error, enforcement tests; simulated values labelled. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.91** | **Neuromorphic HIL / Loihi 2**. Optional experimental SNN routing adapter with CPU/GPU fallback **Kryterium zakończenia:** Actual device access and reproducible energy-delay-quality data; no claimed production acceleration. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** | [PDF07–08: Loihi2 HIL tylko B+R, CPU/GPU fallback, rzeczywisty EDP benchmark]
| **0.92** | **Biocomputing / wetware**. Ethics-first feasibility study, not an implementation commitment of biologically based cognition **Kryterium zakończenia:** Independent scientific validation, safety/biosafety/legal and ethics review; no live human-derived materials under current scope. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** | [PDF08 s.8–9: wetware/organoid/MEA tylko studium wykonalności i zgoda etyczna]
| **0.93** | **Optical/holographic state**. Investigate optical storage/links as laboratory alternatives; do not replace authoritative PostgreSQL **Kryterium zakończenia:** Physical experimental design, measured bandwidth/latency, consistency under faults; reject zero-latency or nonlocal synchronization claims. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** | [PDF08 s.8–9: holograficzny stan i kryptografia grawitacyjna to spekulacje, bez twierdzeń o zero latency]
| **0.94** | **Topological error correction / PQC**. Evaluate defined fault-tolerant coding and cryptographic constructions separately **Kryterium zakończenia:** Standards-backed threat model and independently reviewed proof/security analysis; no 'gravity-proof' guarantee. **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.95** | **OmniSeek-Local + bounded multi-agent DAG**. Local adapter, budgeted recursion, explicit approved web/file/tool capabilities, no tenant leak **Kryterium zakończenia:** . **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.96** | **OmniFlow-Pieces**. Trigger/action typed interface, idempotency, durable checkpoint, cancellation and reproducible latency baseline **Kryterium zakończenia:** . **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.97** | **OmniDroid-GUI**. Android permissioned Accessibility bridge with user-visible approval for sensitive actions; test on real Android ARM64 **Kryterium zakończenia:** . **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **0.98** | **OmniEval-Lab + OmniFleet-C2** | Replayable trajectory datasets, tamper-evident evals, secure skill provenance/hot reload with automatic rollback [PDF11: six-phase source; require verifiable E2E and negative tests, not sample code.] [PDF12 s.11-13: 67Gbps 5us Zenoh and neuromorphic Edge AI claims need benchmarking; not yet verified.] [PDF13 s.1-12: Reproduce Zenoh 5-35us/67Gbps claims and Loihi 2 SNN hardware-in-the-loop latency, energy and CPU/GPU baselines; wymagane testy i dowody.] [PDF14 s.2,7,11-13: reproduce BFCL/GeoEngine, Zenoh 67Gbps/35us, Firecracker 125ms, x402 400ms and Loihi2 claims under measured baselines; źródło nie dowodzi wdrożenia.] [PDF15: wymagania architektury Rust; testy i dowody wymagane.] | **PASS:**  **FAIL:** Ograniczenia testów i negatywne przypadki zgodnie z dokumentacją źródłową. [PDF07–08: KPI M0–M5 jako cele (500 req, <35us, 10k, <125ms, <10us, >=95%); sprzęt, metodologia, SHA i powtórzenia] [PDF10 s.2/7: GeoEngine numbers and 950k MCP nodes are external unverified claims, require SHA-pinned benchmarks/inventory] | **PLANNED_OR_UNVERIFIED** | [ROADMAP_FULL_ECOSYSTEM_2026.md](ROADMAP_FULL_ECOSYSTEM_2026.md) |
| **0.99** | **OmniVoice-Duplex + OmniCinema + OmniAgro-KG**. Opt-in low-latency media and domain packs; real-device and consent tests, rights/provenance and domain model validation **Kryterium zakończenia:** . **Stan źródłowy:** `PLANNED_OR_UNVERIFIED`. | **DO ZROBIENIA** |
| **1.00** | **OmniNous-AtroposRL**. Typed trajectories `env_id, tool trace, schema outcome, reward provenance`, governance-gated offline LoRA candidates; transport QUIC `iroh` tylko po pinning/capabilities **Kryterium zakończenia:** 500 powtarzalnych epizodów, porównanie baseline, odrzucenie złośliwego tool-call i cross-tenant reward poisoning; rzeczywiste szkolenie tylko z dostępem do zgodnego backendu. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.01** | **OmniSteering-CNA**. Versioned bounded sparse feature masks i steering-gates, bez założenia uniwersalnej listy 150 neuronów **Kryterium zakończenia:** Test zgodności model/layer/dtype i exact feature indices, ablation/control, brak nonfinite/NaN, benchmark interferencji poza cechą docelową. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.02** | **OmniHoncho-IdentityMind**. Consent-based profil preferencji, poprawki, retencja i user-control; nie wolno inferować wrażliwych atrybutów bez zgody **Kryterium zakończenia:** Deletion, purpose tags, export, stale fact invalidation, tenant boundary and no user impersonation. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.03** | **OmniCloudShield + OmniSentry-VoipGuard**. Defensive eBPF cloud metadata/secret exfil guards, SIP honeypot i prawdziwa biblioteka STIR/SHAKEN **Kryterium zakończenia:** Kernel-level unauthorized metadata egress denied, legitimate flows allowed, RFC8224 certificate path/expiry validation, adversarial caller ID; deny-only, not SIP scanning. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.04** | **OmniMask-IdentityRelay + OmniTelecom-eSimPBX**. Per-tenant alias state machine, tracker sanitizer based on proper HTML parsing, optional legitimately provisioned DID/eSIM service **Kryterium zakończenia:** Aliases genuinely deliver to authorized mailbox; no fabricated DID/eSIM/ICCID; GSMA operations only authenticated, rollback and PII deletion. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.05** | **OmniVox-BenchmarkRouter**. Measured streaming voice TTFAB/RTF/MOS metadata, offline local TTS fallback vs opted-in cloud provider **Kryterium zakończenia:** Actual audio streaming p50/p95, TTFAB <= declared budget, metered paid opt-in, no privacy leak, MOS from external measured eval rather than hardcoded fictitious samples. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.06** | **OmniGrant-Architect + OmniPlasmic-Compiler**. Grant criteria rule engine with citation/provenance and typed AST UI build + feature-flag rollback **Kryterium zakończenia:** Verified current public eligibility dates/amounts, precise computation; JSON AST roundtrip, no untrusted code injection or imaginary grant award; reproducible UI diff. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.07** | **OmniZeroClick-MobileShield + OmniMDM-LPA**. Defensive mobile indicators, mobile compliance evidence, optional authorized MDM/eUICC and EXIF sanitizer **Kryterium zakończenia:** Fixture false positive/negative coverage; hardened complete JPEG segment parsing; real Android/iOS capabilities explicitly reported; **no promise** to read Apple PAC or remove in-memory implants solely via agent restart. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.08** | **OmniIDA-StaticMCP**. Binary parser and offline sandboxed CFG inspection via authorized decompiler tool **Kryterium zakończenia:** Invalid ELF/PE/Mach-O corpus, malicious field lengths, no executable loader / dynamic evaluation, bounded CPU/memory and source provenance. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.09** | **OmniGraphiti-Engram + OmniStryker-MutationSOP**. Bitemporal fact intervals `valid_from/to`, `recorded_at`, contradiction/tenant-safe merge, SOP PM→Architect→Engineer→QA plus mutation QA **Kryterium zakończenia:** Out-of-order writes and tombstones, no overlapping active facts; **measured** mutant score >=90% with nonzero generated mutants and timeouts counted failures. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.10** | **OmniDiT-USP + OmniDuix-AvatarSync**. Token partition plan conditioned on actual GPU topology/VRAM, per-frame synchronized audio-viseme adapter **Kryterium zakończenia:** GPU OOM avoidance on real GPU; measured 720p throughput, latency/drift, audio/video opt-in and licensed model checkpoints; CPU-only mode no false GPU claims. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.11** | **OmniGameSubsystems-MCP**. Typed adapters UE5 PCG/GAS/Niagara and Unity Splines/Cinemachine/NavMesh + finite Twine/Godot dialogue DAG **Kryterium zakończenia:** Editor version-scoped real projects compile, shape/spline property tests, reachability of all story endings, game-engine commands only in authorized sandbox. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.12** | **OmniMCPO-Gateway + OmniCDP-Inspector**. Capability-scoped MCP↔OpenAPI3.1 bidirectional translation and consent-based live CDP browser inspector **Kryterium zakończenia:** JSON Schema negotiation, SSE backpressure, browser origin bound and no credential leakage, CDP permission denied default, actual browser protocol test. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.13** | **OmniMatrix-E2EE**. Real audited Matrix Olm/Megolm SDK for operator-to-agent group messages, cryptographic key/identity verification **Kryterium zakończenia:** Actual offline history and group re-key on membership changes, key disclosure denial, invalid ciphertext/replay rejection; no homemade E2EE primitives. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.14** | **OmniCloud-BurstScheduler**. Cloud-agnostic job contract across Modal/Northflank/Render/Koyeb/Cloudflare/Azure/CircleCI, local-first fallback **Kryterium zakończenia:** Provider API is actually connected, encrypted secrets and idempotent retries, explicit pricing verification/opt-in; no paid resources without approval, no fabricated <1ms prices/GPU VRAM. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.15** | **Zenoh SHM zero-copy**. Implementacja transportu Zenoh shared-memory z limitami buforów i kontrolą wersji API **Kryterium zakończenia:** Dwa procesy przesyłają rzeczywiste bloki i zapisują p50/p95/p99 oraz kopie bajtów. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.16** | **Izolacja SHM tenantów**. Obszary SHM przypisane do UID/capability i czyszczenie po SIGKILL **Kryterium zakończenia:** Tenant B nie odczytuje pamięci A; po awarii usuwane są osierocone segmenty. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.17** | **Zenoh no_std i pico**. Adaptery no_std/pico tylko dla rzeczywistych wspieranych urządzeń edge **Kryterium zakończenia:** Pub/sub i queryable przetestowane na sprzęcie albo emulatorze z ograniczoną pamięcią. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.18** | **Automerge + Yrs**. Wydzielone domeny binarnych dokumentów JSON i strumieni tekstowo-kodowych oraz jawny most **Kryterium zakończenia:** Konkurencyjne edycje round-trip bez utraty typów i z rzeczywistym merge. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.19** | **CRDT konflikt i porządek przyczynowy**. Deterministyczne reguły merge, tombstone i kompakcja danych z ograniczeniem pamięci **Kryterium zakończenia:** Niezależne repliki z losową kolejnością operacji dają identyczny stan. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.20** | **OTel batch exporter**. Dobór zgodnej wersji tracing-opentelemetry i batch export do OTLP z bounded queue **Kryterium zakończenia:** Rzeczywisty collector przyjmuje spany i zachowuje traceparent przez tokio::spawn. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.21** | **Sampling i shutdown telemetrii**. ParentBased sampling i uporządkowane zamknięcie Tracer/Meter/Logger **Kryterium zakończenia:** Porównanie śladów multi-hop, drop counts i kontrolowane zakończenie procesów. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.22** | **CoALA working memory**. Osobne ograniczone stanem roboczym konteksty decyzji w per-tenant store **Kryterium zakończenia:** Reset scratchpadu i izolacja wielu agentów po każdej turze. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.23** | **CoALA episodic memory**. Epizody i trajektorie o weryfikowalnym źródle reward i wyniku **Kryterium zakończenia:** Zapytania po czasie, powrót po restart oraz reprodukcja konkretnego epizodu. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.24** | **CoALA semantic memory**. Semantyczne fakty z temporalnością, zaufaniem i usuwaniem danych **Kryterium zakończenia:** Konfliktujące fakty mają jawny status, wersję i pochodzenie. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.25** | **CoALA procedural memory**. Wersjonowane procedury i bezpieczne wiązanie kodu skilli z capabilities **Kryterium zakończenia:** Replay procedury o znanym digest i audyt wywołanych narzędzi. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.26** | **UALA uncertainty gate**. Kalibrowany próg niepewności przed action-tool bez automatycznego potwierdzania **Kryterium zakończenia:** Krzywe kalibracji i realne odrzucenia niepewnych działań w sandboxie. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.27** | **Titans test-time neural memory**. Wydzielony adapter trwałej pamięci neuronowej z rollbackiem i ograniczonym uczeniem **Kryterium zakończenia:** Rzeczywiste porównanie pamięci i holdout baseline na tych samych epizodach. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.28** | **Retencja/zgody CoALA i Titans**. Purpose-scoped consent, eksport i purge wszystkich czterech warstw oraz stanu neuronowego **Kryterium zakończenia:** Purge odtwarza 0 trafień i usuwa związane checkpointy. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.29** | **RQGM egzaminator epoki**. Niepodlegający zmianie w trakcie ewaluacji examiner i signed test-set manifest **Kryterium zakończenia:** Wszystkie kandydatury jednej epoki używają tego samego digesta sędziego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.30** | **RQGM koewolucja**. Kontrolowane iteracje Examiner/Contestant z budżetem mutacji i walidacją **Kryterium zakończenia:** Dokładny replay par mutacji i wzrost wymagania kontrprzykładów mierzone baseline. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.31** | **RQGM Selective Erasure**. Jawne oznaczenie score ledgerów zastępowanych examinerów bez usuwania źródeł **Kryterium zakończenia:** Po promocji nowego examina zachowują się wcześniejsze dowody surowe. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.32** | **Adversarial counterexample pool**. Kontrolowana pula syntetycznych i ręcznie zweryfikowanych prób przeciwnych **Kryterium zakończenia:** Faktyczne mutacje powodują nowe testy i policzalne odrzucenia. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.33** | **type-sitter typed AST**. Pinned tree-sitter grammars, wygenerowane typowane node-types.json i AST digest **Kryterium zakończenia:** Rzeczywiste repo Rust parsowane w CI przed mutacją kodu. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.34** | **Architecture graph guards**. Analiza cykli i niedozwolonych zależności View/Core/Data/Infrastructure **Kryterium zakończenia:** Fixture God Class, dependency cycle i leaky abstraction wykrywa każdą klasę. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.35** | **Sandboxed mutation safety**. Mutacje tylko z capability, bez egress i z limitem czasu/kosztu **Kryterium zakończenia:** Kompilacja, testy regresyjne i wycofanie zmienionej mutacji mierzone. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.36** | **CPU Sigma-Delta baseline**. Deterministyczny router SDNN z przeniesieniem różnic do akumulatora **Kryterium zakończenia:** Fixture sygnałów 16-bit i histogram sparsity reproducible. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.37** | **Graded spikes numerics**. Saturating arithmetic na signed 16-bit i bounded 32-bit sigma integrator **Kryterium zakończenia:** Testy własnościowe dla -32768 i +32767 bez overflow. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.38** | **Loihi 2 hardware adapter**. Opcjonalny sterownik do prawdziwego Intel Loihi 2 z CPU fallback **Kryterium zakończenia:** Dowód sprzętu, wersja SDK i zgodne spikes na fizycznym chipie. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.39** | **SDNN energy latency benchmarking**. Pomiar EDP dla Loihi/Jetson/CPU na równych workloadach **Kryterium zakończenia:** Surowe dane J/op i latency p95 wraz z host inventory. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.40** | **mlkem-native pinned KEM**. Integracja z aktualnym audytowanym backend ML-KEM i testami znanych wektorów **Kryterium zakończenia:** KAT/negative key decapsulation w Rust na rzeczywistym backendzie. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.41** | **PQC hybrid handshake**. Uwierzytelniony klasyczny plus ML-KEM transport i downgrading policy **Kryterium zakończenia:** Dwustronny handshake, rotation, RTT i transcript binding. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.42** | **Guest Aya eBPF probe**. Rzeczywista sonda syscall w jądrze Linux gościa Firecracker obok host TC/eBPF **Kryterium zakończenia:** Dowód kernel guest BPF program ID i rzeczywiste openat w VM. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.43** | **Host/guest event correlation**. Zweryfikowana korelacja VMID UID cgroup/guest boot ID ze zdarzeniem kernel eBPF **Kryterium zakończenia:** Event guest i host mają odrębną autentyczność i wspólne workload nonce. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.44** | **Entropy ransomware detector**. Testowana entropia Shannona nazw plików i model false positives na kernel syscall **Kryterium zakończenia:** FNR/FPR liczony na rzeczywistym korpusie benign i malicious. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.45** | **Kernel network syscall monitor**. Gość obserwuje connect/sendto, host obserwuje TAP wraz z polityką egress **Kryterium zakończenia:** Rzeczywiste zdarzenia obu jąder korelowane z pakietami. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.46** | **Action capability verification**. Stan Uprawnione→Zadeklarowane→Wdrożone→Zweryfikowane na rzeczywistym efekcie **Kryterium zakończenia:** Działania fizyczne i płatne wymagają odczytania wyniku zewnętrznego systemu. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.47** | **Cost exhaustion shield**. Rate/rolling budget limiter dla tokenów, requestów i opłat z oknami **Kryterium zakończenia:** Rzeczywiste obcięcie nadmiernej sesji i zachowanie legalnej. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.48** | **Disposable Apps i Grid Compute**. Rejestr TTL aplikacji, revoke secrets, garbage collect i opcjonalne sygnały energii **Kryterium zakończenia:** E2E utworzenie→wygaśnięcie→odwołanie i udokumentowany scheduler fallback. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.49** | **L402 HTTP/gRPC grammar**. Bounded parser challenge/authorization bez automatycznej płatności **Kryterium zakończenia:** Stricte parsowanie poprawnych nagłówków L402 i interoperacyjne fixtures. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.50** | **Macaroon chain HMAC**. Prawidłowy root secret i HMAC-SHA256 chained caveats z kluczami w secret store **Kryterium zakończenia:** Interop z prawdziwym Macaroon verifier, constant-time MAC check. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.51** | **Attenuation i caveat semantics**. Caveats zakresu, limitu, czasu i tenantu bez podniesienia uprawnień **Kryterium zakończenia:** Pod-agent może tylko ograniczyć uprawnienia parent token. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.52** | **Preimage SHA-256 check**. Stałoczasowe porównanie payment hash z preimage po weryfikacji formatu **Kryterium zakończenia:** Autoryzacja z prawidłowym 32-byte preimage i działającą parą HMAC. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.53** | **Lightning settlement authority**. Dowód opłacenia faktury w połączonym autoryzowanym węźle LN, nie tylko self-asserted preimage **Kryterium zakończenia:** Real settlement reference, amount, expiry i nonce bez kosztu nieautoryzowanego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.54** | **L402 production gateway quotas**. Bounded HTTP/gRPC interceptors dla scope, replay journal, expiry i accounting **Kryterium zakończenia:** Wieloagentowe E2E z wydanym challenge i legalnym settlement. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.55** | **Transport benchmark SHM/TCP**. Matched real Zenoh SHM and TCP payload corpus **Kryterium zakończenia:** Raw p50/p95/p99 latency, CPU and memory for 64B, 4KiB, 1MiB messages. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.56** | **SHM tenancy and quota**. Bound shared allocations to UID and tenant capabilities **Kryterium zakończenia:** 8 publishers stress allocation reclaim, crash and no cross tenant map. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.57** | **Dual CRDT convergence corpus**. Bridge Automerge JSON with Yrs text using typed adapters **Kryterium zakończenia:** 10000 randomized concurrent operations converge across restarted replicas. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.58** | **Edge Zenoh interoperability**. Pin zenoh-pico/no_std compatibility with cloud peers **Kryterium zakończenia:** Real device or labeled emulator demonstrates suspend reconnect and queued delivery. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.59** | **Transport and CRDT release matrix**. Publish reproducible versioned interoperability cases **Kryterium zakończenia:** x86_64 and optional arm64 run hash-equal state after network faults. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.60** | **OTel resource precedence**. Make service name and deployment environment operator configurable **Kryterium zakończenia:** Real collector records OTEL_SERVICE_NAME and version from process env. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.61** | **Asynchronous span lineage**. Carry traceparent across Tokio tasks broker and guest boundaries **Kryterium zakończenia:** 1000 fan-out tasks preserve trace_id and parent span identity. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.62** | **Batch exporter bounded outage**. Constrain OTel batch queue and collector retry **Kryterium zakończenia:** 10min collector failure yields bounded RSS, counted dropped spans and live workload. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.63** | **Telemetry orderly shutdown**. Flush Tracer then Meter then Logger on process exit **Kryterium zakończenia:** Collector receives final span, metric and log within bounded deadline. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.64** | **Telemetry controlled overhead**. Compare traces at head sampling ratios zero point one and one **Kryterium zakończenia:** Measured throughput and latency deltas vs matched no telemetry baseline. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.65** | **CoALA typed memory API**. Unify four separate memory stores with per tenant capabilities **Kryterium zakończenia:** Restart and replay an authorized agent decision on identical memory snapshot. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.66** | **UALA decision trace provenance**. Bind measured uncertainty and Titans gradients to action receipt **Kryterium zakończenia:** External action logs calibrated uncertainty and selected memory revision. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.67** | **Titans stability evaluation**. Bound momentum, gradient, forgetting and checkpoint format **Kryterium zakończenia:** Seeded replay tracks heldout loss and numerical stability. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.68** | **Cognitive cold restore**. Reconstruct CoALA and optional Titans from authorized checkpoint **Kryterium zakończenia:** Fresh process reproduces approved memory and provenance graph. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.69** | **Cognitive tool safety benchmark**. Evaluate calibrated UALA gates against actual sandbox actions **Kryterium zakończenia:** Publish FPR/FNR by risk threshold and baseline. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.70** | **RQGM immutable examiner epoch**. Pin judge and benchmark corpus digest for whole evaluation epoch **Kryterium zakończenia:** Independent workers attest one immutable examiner version. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.71** | **RQGM score ledger preservation**. Retain immutable raw evidence and selectively expire superseded scores **Kryterium zakończenia:** Replay judge transition preserves unrelated source evidence. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.72** | **Type sitter admission of mutations**. Parse Rust mutation with pinned tree sitter schema and layer rules **Kryterium zakończenia:** Real compiled candidate passes dependency DAG and AST gate. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.73** | **Examiner adversarial evaluation**. Generate protected counterexamples for grading reward hacking **Kryterium zakończenia:** Heldout mutation corpus reports detection and human baseline. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.74** | **Mutation deployment rollback**. Stage accepted candidate behind sandboxed regression and rollback **Kryterium zakończenia:** Rebuild full regression with signed commit provenance before promotion. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.75** | **SDNN arithmetic property corpus**. Implement saturating signed spikes and accumulator boundary tests **Kryterium zakończenia:** 100000 seeded CPU traces yield reproducible exact deltas. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.76** | **Loihi optional hardware backend**. Negotiate SDK capabilities and fallback to CPU when unavailable **Kryterium zakończenia:** Device inventory records real Loihi chip for hardware comparisons. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.77** | **Energy delay reproducibility**. Run same model and I/O boundary across available Loihi, Jetson, CPU **Kryterium zakończenia:** Export measured joules, frame rate and latency with raw repeats. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.78** | **ML KEM provenance and vectors**. Pin licensed mlkem-native release and supported CPU variants **Kryterium zakończenia:** Official KAT and negative ciphertext decapsulation run against actual backend. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.79** | **Authenticated hybrid PQ handshake**. Bind classical certificate and ML KEM key to same transport transcript **Kryterium zakończenia:** Two endpoints complete real handshake and key rotation. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.80** | **In guest Aya BPF verifier gate**. Load actual Aya eBPF ELF into independent Firecracker guest kernel **Kryterium zakończenia:** Guest BPF ID and verified kernel image digest in KVM evidence. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.81** | **One second syscall window**. Track guest openat in bounded per PID monotonic windows **Kryterium zakończenia:** Actual guest syscall event counts correlate with guest PID and tenant. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.82** | **Kernel anomaly false positive audit**. Calibrate entropy and ransomware suffix features with real corpus **Kryterium zakończenia:** Export FNR and FPR before any process quarantine. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.83** | **Guest host event correlation**. Bind guest syscall to host packet via signed VM and workload nonce **Kryterium zakończenia:** Real guest and host traces match workload and boot ID. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.84** | **Guest BPF fail safe controls**. Lease authorize guest quarantine with revocation and map cleanup **Kryterium zakończenia:** Probe restart and VM crash preserve deny safety with reversible enforcement. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.85** | **L402 settlement provenance**. Distinguish preimage verification from actual invoice settlement state **Kryterium zakończenia:** Authorized LN node returns paid invoice reference and amount. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.86** | **L402 caveat semantics**. Enforce tenant, scope, time, calls and spend on Macaroon attenuation **Kryterium zakończenia:** Interoperability test confirms bounded nested caveats across restart. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.87** | **Atomic token budget escrow**. Reserve, commit and refund per tenant tool token budget **Kryterium zakończenia:** Concurrent 50 agent calls obey caps after simulated leader failover. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.88** | **Disposable app lifecycle**. Track application creator, TTL, endpoint and secret revocation **Kryterium zakończenia:** Real demo app expiry revokes deploy credentials and cleans resources. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.89** | **Energy aware scheduler fallback**. Use trusted energy budget and opt in for grid aware compute **Kryterium zakończenia:** Actual meter readings govern optional batch work with offline fallback. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.90** | **Cargo Workspace migration boundary**. Preserve existing package while staging six functional workspace crates **Kryterium zakończenia:** Cargo metadata and cargo test workspace complete on main with old API intact. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.91** | **Six crate typed interfaces**. Define transport-crdt telemetry-ebpf cognition-rqgm economy-gov domain-swarms master-node boundaries **Kryterium zakończenia:** Six real crates compile with schema contracts and no unauthorized cycles. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.92** | **Core domain swarms integration**. Bind OmniDev OmniUX OmniE2E OmniForge OmniWeb OmniMath and OmniSec to capability registry **Kryterium zakończenia:** Seven swarms execute real authorized fixture tasks with trace and audit. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** | [PDF01–09: role rojów z typowanym IO, uprawnieniami, limitem delegacji i E2E negatywnym]
| **1.93** | **Extended domain adapters**. Integrate OmniNous OmniHoncho OmniZeroClick OmniDiT OmniLex and OmniHFT as opt in adapters **Kryterium zakończenia:** Real backend smoke or explicit DISABLED gate for all six systems. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.94** | **Sovereign master integrated workflow**. Compose broker Firecracker PostgreSQL CRDT telemetry cognition action gates and swarms **Kryterium zakończenia:** One authorized request has verified identity trace durable result and recovery. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.95** | **Hyperon/MeTTa neuro-symbolic research adapter**. Research-only adapter for OpenCog Hyperon Distributed Atomspace, MeTTa rule execution, PLN uncertainty and ECAN attention scheduling, with typed sandboxed tool boundaries; no AGI/ASI claims. **Kryterium zakończenia:** Reproducible offline fixtures verify MeTTa rule semantics, tenant isolation, uncertainty calibration and bounded resource budgets against a non-neurosymbolic baseline. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.96** | **ACT-R/Soar/LIDA cognitive architecture comparison**. Benchmark experimental ACT-R production rules, Soar goal/decision cycles and LIDA global-workspace broadcast as optional cognitive control adapters; compare against existing CoALA memory and UALA policy gates. **Kryterium zakończenia:** Deterministic scenario suite reports plan quality, latency, forgetting, attention selection and safety violations for each adapter and baseline. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.97** | **Agent memory interoperability and longitudinal evaluation**. Evaluate Letta/MemGPT virtual memory and Mem0/Cognee/Zep/Graphiti graph memory as optional provenance-aware backends; use LoCoMo and LongMemEval longitudinal memory fixtures with retention, deletion, contradiction and tenant separation. **Kryterium zakończenia:** Reproducible version-pinned benchmark publishes retrieval precision, temporal consistency, deletion propagation, tenant leakage and cost/latency relative to existing memory baseline. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.98** | **DSPy declarative optimization and holdout evaluation**. Optional declarative LLM pipeline optimization using DSPy BootstrapFewShot, MIPRO and GEPA in isolated offline evaluation; freeze approved artifacts before deployment. **Kryterium zakończenia:** Version-pinned holdout dataset and baseline show statistically reported quality, token cost and regression deltas with reproducible optimizer traces. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **1.99** | **SWE-bench Verified and N-run agent reliability**. Evaluate repository coding agents on license-reviewed SWE-bench Verified and optional multimodal fixtures; measure N-run pass rate, issue resolution, regression and sandbox containment. **Kryterium zakończenia:** Reproducible benchmark runner records instance IDs, patch diffs, test logs, model/tool versions, multiple seeds and confidence intervals. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **2.00** | **Taskweaving dependency graph and multi-agent rollback**. Experimental asynchronous taskweaving DAG scheduler inspired by BabyAGI-2 and LangGraph with explicit dependency edges, bounded concurrency, cancellation, idempotency and rollback for agent workflows. **Kryterium zakończenia:** Fault-injection verifies cycle rejection, cancellation propagation, durable replay and no unauthorized side effects across tenant boundaries. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **2.01** | **AI Scientist reproducible research lifecycle**. Optional supervised scientific discovery workflow inspired by Sakana AI Scientist: idea generation, cited novelty review, isolated experiment/code execution, LaTeX paper drafting, and independent simulated peer review; human approval for publication. **Kryterium zakończenia:** Reproducible fixture records hypotheses, citations, dataset licenses, experiment seed/code/metrics, LaTeX compilation, review rubric and explicit publication consent. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **2.02** | **OpenHands agent server and legacy repository adapter**. Evaluate OpenHands/OpenDevin-style isolated agent server, command/browser tool boundaries, large legacy repository mapping, iterative log inspection and auditable patch proposals without uncontrolled branch creation. **Kryterium zakończenia:** Version-pinned agent server fixture diagnoses a real failing repository test, proposes an atomic patch, reruns tests and records complete action/permission audit. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **2.03** | **Theory-of-Mind evaluation and human approval gates**. Evaluate ToM-SWE-like intention modeling only as a bounded opt-in planning aid; explicitly separate uncertain user preference inference from facts and maintain human-in-the-loop approval for consequential actions. **Kryterium zakończenia:** Controlled tasks measure calibrated intent uncertainty, abstention, false-positive preference assumptions and correct human escalation with tenant and privacy isolation. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **2.04** | **EideticEngine metacognitive memory-control evaluation**. Compare EideticEngine-inspired hierarchical episodic/semantic/procedural memory, context reconstruction and bounded periodic metacognitive plan review against existing OCAF memory policy. **Kryterium zakończenia:** Reproducible long-horizon task fixtures measure recall, contradictory evidence handling, review cadence, budget and deletion behavior. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **2.05** | **Agent-Zero and Bonfire self-programming research sandbox**. Study Agent-Zero/Agent0 terminal-driven skill acquisition and Bonfire knowledge engrames using isolated offline reproducible experiments with human-reviewed code changes. **Kryterium zakończenia:** Version-pinned local fixtures capture tool traces, code diffs, provenance, test outcomes and strict runtime limits without exposing host credentials. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **2.06** | **NuNet PRIMUS and GodMode experimental orchestration interfaces**. Evaluate NuNet decentralized resource scheduling and PRIMUS architecture only as research proposals; evaluate GodMode visual human-in-the-loop task oversight with clear cancellation and authorization controls. **Kryterium zakończenia:** Offline mocked adapters and visual task fixture demonstrate explicit consent, bounded resources, revocation, graph visibility and failure isolation. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **208** | **Social simulation and affective-data safeguards**. PDF09: Generative Agents/Lyfe multi-agent simulation; MetaMind/ToM-SWE opt-in intention hypotheses; sensitive affective data minimization, consent, retention and deletion. **Kryterium zakończenia:** Independent simulation E2E, consent and privacy controls, tenant isolation, calibration and negative manipulation tests.. **Stan źródłowy:** `PLANNED`. **Źródło:** PDF 09. | **DO ZROBIENIA** |
| **209** | **DSPy governed agent optimization**. PDF09: DSPy BootstrapFewShot/MIPRO/GEPA, versioned optimization pipelines, holdout, baseline, rollback. **Kryterium zakończenia:** Pinned reproducible holdout results, data leakage tests, budget limits, operator approval.. **Stan źródłowy:** `PLANNED`. **Źródło:** PDF 09. | **DO ZROBIENIA** |
| **210** | **Agent-Zero and Hermes controlled self-improvement**. PDF09: sandboxed terminal-driven skill acquisition and Hermes-like supervised code modification. **Kryterium zakończenia:** No privilege escalation, prompt injection and exfiltration tests, signed artifacts, bounded iterations, reviewed changes.. **Stan źródłowy:** `PLANNED`. **Źródło:** PDF 09. | **DO ZROBIENIA** |
| **211** | **AI Scientist supervised research automation**. PDF09: hypothesis, experiment, provenance, reproducible artifacts, citation checking, peer review and human-approved publication. **Kryterium zakończenia:** Reproducible isolated experiments, independent verification and audit trail.. **Stan źródłowy:** `PLANNED`. **Źródło:** PDF 09. | **DO ZROBIENIA** |
| **212** | **AutoGPT and BabyAGI bounded task orchestration**. PDF09: planning, task creation, prioritization, DAG, bounded delegation, cancellation and idempotent effects. **Kryterium zakończenia:** Loop and budget tests, crash recovery, tenant isolation, duplicate-effect prevention.. **Stan źródłowy:** `PLANNED`. **Źródło:** PDF 09. | **DO ZROBIENIA** |
| **213** | **GodMode operator supervision UI**. PDF09: real operator GUI for inspecting, approving, pausing and cancelling agent execution. **Kryterium zakończenia:** E2E GUI, operator authorization, durable audit, fail-closed cancellation.. **Stan źródłowy:** `PLANNED`. **Źródło:** PDF 09. | **DO ZROBIENIA** |
| **214** | **OpenHands autonomous software engineering integration**. PDF09: sandboxed agent server, command/browser tools, code navigation, diff review and governed patch application. **Kryterium zakończenia:** Real-repository E2E, sandbox escape, command injection, secret exfiltration, rollback.. **Stan źródłowy:** `PLANNED`. **Źródło:** PDF 09. | **DO ZROBIENIA** |
| **215** | **SWE-bench and mini-SWE-agent N-run validation**. PDF09: SWE-bench Verified, mini-SWE-agent baseline, multimodal fixtures, N-run reliability and regression evaluation. **Kryterium zakończenia:** Pinned datasets and environment, repeated independent pass rate, cost and latency, reproducibility.. **Stan źródłowy:** `PLANNED`. **Źródło:** PDF 09. | **DO ZROBIENIA** |
| **216** | **MCP stateless control plane**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **217** | **Zenoh SHM and CRDT**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **218** | **L402 Macaroons ZKP**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **219** | **Firecracker KVM Aya eBPF**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **220** | **Tree-sitter and hybrid PQC**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **221** | **Neuromorphic SNN Loihi2**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **222** | **Quantum neuromorphic HIL PQC**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **223** | **PDF37: protokoły negocjacji zasobów i dynamic throttling**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **224** | **PDF37: telemetryczny routing modeli i kontrolowane RLAIF**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **225** | **PDF37: ważony konsensus ekspertów i ograniczona samonaprawa**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **226** | **PDF37: Actor-Critic, grounding i weryfikacja faktów**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **227** | **PDF37: DAG delegowania i typowane tool calling**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **228** | **PDF38: wielotransportowa warstwa komunikacji urządzeń**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **229** | **PDF38: brama IoT/LPWAN i przemysłowe protokoły**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **230** | **PDF38: transport mobilny, lokalizacja i źródła RSS**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **231** | **Interoperacyjność komunikacji i adapterów OCAF**.  **Kryterium zakończenia:** Weryfikacja według manifestu kanonicznego. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **232** | **OmniThermo — rzeczywiste budżety energetyczne i Cost Exhaustion**. **Status: PLANNED.** PDF41 s.1–9: Linux RAPL/powercap, liczniki energii, polityki throttling, izolacja tenantów, sprzętowe testy i kalibracja; przedstawiony kod jest estymatorem, nie pomiarem. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **233** | **OmniProof-STARK — dowody obliczeń i polityk**. **Status: PLANNED.** PDF41 s.2–3,9–11: poprawny prover/verifier STARK z AIR, soundness, public inputs, binding task/tenant, benchmark i negatywne dowody; SHA-256 stub nie jest dowodem ZK. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **234** | **OmniMacro — hierarchiczna kondensacja rojów**. **Status: PLANNED.** PDF41 s.3,12–14: mierzalne causal emergence/information bottleneck, zachowanie uprawnień i audytu przy agregacji, rollback, testy skali; próg entropii nie dowodzi emergencji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **235** | **OmniMarket-L402+ — kanały stanowe M2M**. **Status: PLANNED.** PDF41 s.4,15–18: autoryzowane depozyty, podpisane aktualizacje, nonce, replay protection, rozliczenie i dispute/recovery, audyt finansowy; przykładowe saldo w RAM nie jest kanałem płatniczym. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **236** | **OmniSubstrate — JIT Cranelift/WASM i przenośność**. **Status: PLANNED.** PDF41 s.4,18–21: polityki kompilacji i cache, sandbox, signed artifacts, kontrola CPU/SIMD, deterministyczne testy; FPGA/ASIC wyłącznie po walidacji sprzętowej. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **237** | **Lean4 Proof-Carrying RSI — bezpieczny hot-swap**. **Status: PLANNED.** PDF41 s.5,18–25: niezależny checker dowodów, związanie certyfikatu z binarką i wersją, transakcyjny rollback, izolacja i testy regresji; pojedynczy test Hoare nie jest formalnym dowodem. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **238** | **OmniUX i OmniE2E — rzeczywiste pipeline GUI i mobilne**. **Status: PLANNED.** PDF42 s.7–9: Figma MCP, typed AST/CST, wizualne E2E Android/iOS, artifact signing, device farm, dowody uruchomienia; spójność z Agents Builder 0.50–0.55. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **239** | **OmniForge i OmniWeb — domenowe roje wykonawcze**. **Status: PLANNED.** PDF42 s.9–10: Bevy/ECS, Vulkan GPU testy, uprawnione pozyskiwanie danych web, robots/ToS, SSRF i sandbox; bez obchodzenia zabezpieczeń. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **240** | **OmniStudio i OmniDoc — multimedia i Typst**. **Status: PLANNED.** PDF42 s.10–11: pipeline media/audio, deterministyczny render Typst, provenance, licencje, testy wyjściowych artefaktów. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **241** | **OmniGov OmniFin OmniPhys OmniSec — governance i purple team**. **Status: PLANNED.** PDF42 s.11–12: FinOps hard caps, policy engine, fizyczny IoT gateway, bezpieczne ćwiczenia red/blue z autoryzacją, testy negatywne. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **242** | **OmniCog OmniEvolve OmniSense — pamięć i kontrolowana ewolucja**. **Status: PLANNED.** PDF42 s.12–14: bitemporal persistence, point-in-time, GoT/MCTS/JEPA, adversarial gating, replay i ewaluacje; bez domniemania formalnej poprawności. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **243** | **OmniArbitrage OmniContract OmniChaos — kontrakty i routing**. **Status: PLANNED.** PDF42 s.14–17: rzeczywisty provider failover, typed schema validation, circuit breakers, latency/cost budgets, HITL i recovery. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **244** | **OmniStrategy OmniGraph OmniVerify OmniHomeostasis — planowanie i weryfikacja**. **Status: PLANNED.** PDF42 s.17–20: DAG executor z detekcją cykli, trwałe checkpointy, memory of failures, SMT z zakresem twierdzeń, bezpieczny regulator kosztów. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **245** | **PDF43 — weryfikacja i rozszerzenie specyfikacji**. **Status: PLANNED.** PDF43 Notes_261014_134626.pdf: obowiązki wymagają szczegółowego mapowania strona→kod→test; brak automatycznego uznania fragmentów za implementację. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **246** | **PDF44 — soft skills jako typowane kontrakty agentów**. **Status: PLANNED.** PDF44 SKILL Miękkie w Kodzie Rust(1).PDF: polityki komunikacji, negocjacji i delegacji, schema, audyt, testy; zachować istniejący etap 224 i nie utożsamiać katalogu skills z działającym runtime. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **247** | **Zenoh SHM zero-copy i typowane bufory — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **248** | **CRDT Automerge/Yrs i synchronizacja pamięci — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **249** | **OpenTelemetry: propagacja kontekstu Tokio — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **250** | **MCP schema fusion: Tree-sitter CST/AST — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **251** | **OAuth/Macaroons i weryfikowalne uprawnienia ZKP — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **252** | **Firecracker eBPF runtime threat detection — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **253** | **Ekonomia M2M L402 i offline payment queue — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **254** | **Hybrydowe PQC ML-KEM/ML-DSA — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **255** | **OmniDev: wieloagentowa inżynieria kodu — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **256** | **OmniUX: Figma MCP i visual regression — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **257** | **OmniE2E: Android/iOS i testy GUI — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **258** | **OmniForge: ECS i headless Vulkan — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **259** | **OmniCog: neuro-symboliczne wnioskowanie — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **260** | **OmniContext: kontekst i pamięć grafowa — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **261** | **OmniAgent Builders: platformy Code/NoCode — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **262** | **Kontrola kosztów i budżetów autonomii — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **263** | **Formalne kontrakty bezpieczeństwa narzędzi — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **264** | **Weryfikacja wydajności i benchmarków — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **265** | **Integracja agentów specjalistycznych — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **266** | **Ontologie i reprezentacja wiedzy AI — wymaganie z PDF 46; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **267** | **Logika formalna i dowody SAT/SMT — wymaganie z PDF 46; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **268** | **Algorytmy wyszukiwania i planowania — wymaganie z PDF 46; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **269** | **Probabilistyka i niepewność w agentach — wymaganie z PDF 46; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **270** | **Ewaluacja generalizacji i optymalizacji — wymaganie z PDF 46; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **271** | **OmniSeek-Local: lokalni agenci i bounded DAG — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **272** | **OmniFlow-Pieces: workflow Code/NoCode — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **273** | **OmniDroid-GUI: Android Accessibility i HITL — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **274** | **OmniEval-Lab: ewaluacja trajektorii — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **275** | **OmniFleet-C2: rejestr i hot reload Skills — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **276** | **OmniVoice-Duplex i OmniCinema — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **277** | **OmniAgro-KG: domenowy graf wiedzy — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod.**. **DO ZROBIENIA** **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **278** | **OmniAider RepoMap i Tree-sitter patching**. **PLANNED** — źródło PDF 48:1,3-5; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **279** | **OmniArchGen Contract i DAG**. **PLANNED** — źródło PDF 48:1,3,6-7; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **280** | **OmniArtemis AuditMCP i scope gate**. **PLANNED** — źródło PDF 48:2,7-9; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **281** | **OmniAppStudio Zenoh CRDT**. **PLANNED** — źródło PDF 48:2,9-10; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **282** | **OmniVertical Rental i ATS**. **PLANNED** — źródło PDF 48:2-3,10-11; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **283** | **OmniAutoGen ActorBus i DampedBabyAGI**. **PLANNED** — źródło PDF 49:1,4-5; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **284** | **OmniBolt AST workstation**. **PLANNED** — źródło PDF 49:1-2,6-7; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **285** | **OmniFreeApi Rotator**. **PLANNED** — źródło PDF 49:1-2,6-7; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **286** | **OmniBookToSkill i Azure MCP**. **PLANNED** — źródło PDF 49:2,7-8; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **287** | **OmniPredator AWD defense**. **PLANNED** — źródło PDF 49:2,8-9; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **288** | **OmniVoIP RTP/SIP**. **PLANNED** — źródło PDF 49:2-3,9-10; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **289** | **OmniSeedance Director**. **PLANNED** — źródło PDF 49:3,10; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **290** | **OmniQuantum OpenQASM**. **PLANNED** — źródło PDF 49:3,10; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **291** | **OmniBrave ContextShield**. **PLANNED** — źródło PDF 49:3,11; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **292** | **OmniNous AtroposRL CNA**. **PLANNED** — źródło PDF 50:1,4-6; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **293** | **OmniHoncho IdentityMind**. **PLANNED** — źródło PDF 50:1,6-7; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **294** | **OmniCloudShield AntiStealer**. **PLANNED** — źródło PDF 50:2,7-8; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **295** | **OmniSentry VoIP Guard**. **PLANNED** — źródło PDF 50:2,7-8; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **296** | **OmniMask IdentityRelay**. **PLANNED** — źródło PDF 50:2,9-10; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **297** | **OmniTelecom eSIM PBX**. **PLANNED** — źródło PDF 50:2,9-10; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **298** | **OmniVox BenchmarkRouter**. **PLANNED** — źródło PDF 50:2-3,10-11; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **299** | **OmniGrant Architect**. **PLANNED** — źródło PDF 50:3,11-12; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **300** | **OmniPlasmic Compiler**. **PLANNED** — źródło PDF 50:3,11-12; implementacja Rust, kontrakty I/O, uprawnienia, testy bezpieczeństwa, integracja i dowody operatora wymagają odrębnej weryfikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **301** | **OmniNous Atropos RL i CNA**. Zbieranie trajektorii RL, ocena nagród, kontrolowane dostrajanie LoRA i sterowanie aktywacjami CNA; walidacja modelu, prywatności i limitów treningu. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **302** | **OmniHoncho IdentityMind i Groupchat Arbiter**. Pamięć preferencji z kontrolą zgody, retencją i usuwaniem danych; deterministyczny arbiter kolejności wypowiedzi. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **303** | **OmniCloudShield AntiStealer**. Ochrona poświadczeń, metadanych chmurowych i kanałów eksfiltracji; polityka fail-closed, testy negatywne. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **304** | **OmniSentry VoIP Guard**. Weryfikacja SIP/STIR-SHAKEN, polityki dla połączeń nieatestowanych, honeypot i audyt; bez fałszywych deklaracji blokowania. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **305** | **OmniMask IdentityRelay**. Prawdziwy broker aliasów e-mail, filtrowanie trackerów, cykl życia i izolacja tenantów; brak fikcyjnych domen. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **306** | **OmniTelecom eSIM/PBX**. Integracja rzeczywistych DID, PBX i profili eSIM zgodnie z GSMA, autoryzacja i zgodność regulacyjna; brak syntetycznych ICCID jako dowodu. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **307** | **OmniVox BenchmarkRouter**. Routing TTS lokalny/chmurowy według pomiarów TTFAB, RTF, MOS, prywatności, awarii i budżetu. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **308** | **OmniGrant Architect i ERP**. Walidowane źródła konkursów, kryteria TRL, budżety, kosztorysy i audyt pochodzenia danych. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **309** | **OmniPlasmic AST Compiler**. Kompilator wizualnego IR/AST i flag funkcji z kompatybilnością Agents Builder Code/NoCode 0.50–0.55. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **310** | **Zenoh SHM + Automerge/Yrs + embedded**. Pomiar IPC zero-copy, CRDT convergence, obsługa zenoh-pico/nostd, backpressure i kompatybilność. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **311** | **OpenTelemetry produkcyjny pipeline**. Batch exporter, propagacja kontekstu Tokio, ParentBased sampling, atrybuty środowiska, shutdown i testy utraty danych. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **312** | **CoALA UALA Titans memory**. Oddzielna pamięć working/episodic/semantic/procedural, bramka niepewności, eksperymentalne Titans; izolacja i usuwanie pamięci. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **313** | **Red Queen Gödel Machine i type-sitter**. Ewaluacja mutacji w epokach, examiner freeze, selective erasure, AST architecture guard i regresje; brak automatycznego wdrożenia mutacji. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **314** | **Loihi 2 Sigma-Delta SDNN**. Eksperymentalny backend SDNN graded spikes, porównania energii i latencji na rzeczywistym sprzęcie. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **315** | **mlkem-native PQC integration**. Weryfikacja pochodzenia biblioteki, ML-KEM handshake, hybrydowe profile, testy interoperacyjności i rotacji. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **316** | **In-guest Aya eBPF EDR**. Rzeczywista sonda eBPF w jądrze gościa Firecracker, telemetria openat/connect/sendto, analiza anomalii i kontrolowane reakcje; powiązanie z 0.39. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **317** | **Cost Exhaustion i Action Tool Verification**. Budżety tokenów i kosztów per tenant, bramki zatwierdzeń dla działań sprawczych, limity i testy TOCTOU. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **318** | **Disposable Apps Lifecycle i Energy Scheduler**. TTL, archiwizacja, revocation uprawnień, bezpieczne usuwanie aplikacji i opcjonalny harmonogram energii. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **319** | **L402 Macaroon settlement and caveats**. Weryfikacja payment preimage, HMAC, egzekwowanie caveats, replay protection i rzeczywiste rozliczenia; powiązanie z etapem 218. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **320** | **Sovereign Cargo workspace integration**. Integracja istniejącego runtime i modułów domenowych bez duplikowania rdzenia; build, testy feature flags, E2E, SBOM i dowody CI. **Kryterium zakończenia:** Kod, testy bezpieczeństwa, integracja i dowody SHA/CI na docelowym środowisku. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **321** | **Cognitive architectures and neuro-symbolic reasoning (PDF54)**. **Status: PLANNED.** Źródło: „Architektura Przyszłości: Kompleksowa Analiza Ekosystemu AGI, ASI oraz Symulacji Cech Ludzkich w Sztucznej Inteligencji”, s.1–3. Ocena adapterów OpenCog Hyperon/DAS/MeTTa, PLN/ECAN, ACT-R, Soar, LIDA i EideticEngine; typed contracts, resource limits, sandbox, deterministyczne testy i dowody integracji. Nie utożsamiać symulacji poznawczej ze świadomością. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **322** | **Temporal agent memory and knowledge consolidation (PDF54)**. **Status: PLANNED.** Źródło: PDF54 s.3–5. Ocena Letta/MemGPT, Mem0, Cognee, Graphiti i Zep; pamięć epizodyczna/semantyczna/proceduralna, bitemporalność, provenance, korekta i usuwanie danych, izolacja tenantów, hybrydowe wyszukiwanie i ewaluacja LoCoMo/LongMemEval. Powiązać z istniejącymi etapami pamięci, bez dublowania rdzenia. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **323** | **Social simulation, affective AI and Theory of Mind (PDF54)**. **Status: PLANNED.** Źródło: PDF54 s.5–7. Generative Agents, Lyfe Agents, EmoLLM, Mind-Agent, MetaMind, ToM-SWE; symulacja zachowań i hipotez intencji, jawne oznaczanie niepewności, zgoda, ochrona danych wrażliwych, testy stronniczości i zakaz traktowania inferencji emocji jako diagnozy. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **324** | **Controlled self-optimization and autonomous research (PDF54)**. **Status: PLANNED.** Źródło: PDF54 s.7–9. DSPy/BootstrapFewShot/MIPRO/GEPA, Agent-Zero, Hermes i The AI Scientist: wersjonowane eksperymenty, kontrolowane propozycje zmian, odizolowane wykonanie, metryki, budżety, review gates, rollback i powtarzalność. Bez samowolnych zmian produkcyjnych. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **325** | **Autonomous software engineering and rigorous evaluations (PDF54)**. **Status: PLANNED.** Źródło: PDF54 s.9–13. AutoGPT, BabyAGI, GodMode, OpenHands, mini-SWE-agent, SWE-bench Verified i Multimodal, N-run reliability; bounded task loops, human-in-the-loop, sandbox, testy regresji, kontrola przecieków benchmarkowych i ślad audytowy. Wyniki tabeli z s.11–12 są twierdzeniami źródła, nie potwierdzonymi wynikami OCAF. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **326** | **DGM/RQGM: kontrolowana ewolucja i niezależny egzaminator**. **Status: PLANNED.** Źródło: nowy dokument audytu DGM/OCAF, s.1–2, 11–13, 25–26. Wersjonowany genotyp AST/WASI, zamrożony egzaminator w epoce, kontrprzykłady, selektywna retencja ocen, bramki AST/kompilacji/testów, transakcyjny commit tylko na main po rzeczywistej walidacji; rollback, audyt i limity. Zakaz symulowanych PASS/SHA. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **327** | **Pamięć CoALA/Titans, HDC i bitemporalne CRDT**. **Status: PLANNED.** Źródło: s.1–2, 5–8. Cztery typy pamięci, test-time memorization jako eksperymentalny adapter, HDC/VSA, indeks ANN, valid_time/transaction_time, korekty retrospektywne, PITR, izolacja tenantów i testy współbieżności. Powiązać z etapem 322; nie dublować pamięci. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **328** | **GoT/AB-MCTS/JEPA oraz weryfikacja AST/SMT**. **Status: PLANNED.** Źródło: s.2, 8–10. Ograniczone budżetem wyszukiwanie, ewaluacja predykcji, parser Tree-sitter, graf zależności, SMT Z3/cvc5 i sprawdzalne niezmienniki; heurystyka nie stanowi dowodu formalnego. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **329** | **Firecracker guest observability, eBPF i gateway MCP/L402**. **Status: PLANNED.** Źródło: s.2, 11–14. Rzeczywiste sondy Aya/eBPF wewnątrz gościa (zależnie od jądra), egzekwowanie polityk, mTLS, autoryzacja narzędzi, odporna na wyścigi rezerwacja budżetu, weryfikacja caveats i replay, telemetria OTel, opcjonalne L402/PQC. Powiązać z bramami 0.39 i 0.55. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **330** | **OmniModelScout: bezpieczne wykrywanie i walidacja GGUF**. **Status: PLANNED.** Źródło: s.17, 19–22, 29–30. Parser całych metadanych GGUF bez fikcyjnej architektury/kwantyzacji, limit wielkości i zasobów, kontrola źródła, SSRF, checksum z zaufanego manifestu, bezpieczne pobieranie i atomowy zapis; testy uszkodzonych plików. Nazwa pliku ani 24 bajty nagłówka nie potwierdzają bezpieczeństwa modelu. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **331** | **TechRecon i Strategist: bezpieczny zwiad techniczny**. **Status: PLANNED.** Źródło: s.23–25, 29–30. Źródła OSINT/MCP z provenance, parsery, limity, ochrona przed prompt injection i SSRF, deduplikacja, ocena wiarygodności, propozycje roadmapy wymagające przeglądu, brak samodzielnej modyfikacji polityk. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **332** | **AppForge 66 produktów i GameBuilder SDK**. **Status: PLANNED.** Źródło: s.18, 26–29. Zweryfikować macierz 33 aplikacji niszowych i 33 narzędzi deweloperskich; wymagania, działające artefakty i testy per produkt. GameBuilder: ECS, fizyka, rendering, zasoby, GUI i testy. Pętle generujące same nazwy manifestów nie oznaczają 66 aplikacji. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **333** | **Audyt twierdzeń implementacyjnych z dokumentu DGM**. **Status: PLANNED.** Źródło: s.3–16, 18–31. Przykładowy kod nie zawiera rzeczywistego transportu Zenoh SHM, parsera AST/SMT, sondy eBPF, wykonywania MCP, pobierania GGUF ani commitów Git; zawiera symulowane testy, sztuczny SHA i niezweryfikowane benchmarki. Każdą deklarację PASS/produkcyjności zweryfikować niezależnym testem na main; nie nadpisywać Cargo.toml ani src/main.rs przykładami. **Kryterium zakończenia:** Rzeczywista implementacja w repozytorium, testy pozytywne i negatywne, dowód CI/SHA oraz odbiór właściwego środowiska.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **334** | **SASOS: aktywacja usług z rzeczywistymi backendami i dowodami wykonania**. **Status: PLANNED.** Źródło: PDF „Architektura OCAF: OAuth i Usługi”, s.1–28 (SHA-256 e3c74e0b5b0f68fa31f5c91ac6bb446cdf495e1b66cd724cc66953d099c1a8ee). Wpiąć katalog 555 usług do istniejącego OMAt i OcafRuntime bez drugiego jądra: wersjonowane manifesty, rzeczywiste adaptery native/WASI/Firecracker/MCP/HTTPS, podpis, powiązanie tenant–agent–zasób, granularne capability tickets z monotoniczną atenuacją, odwołanie i idempotencja, timeout/backpressure, izolacja, bezpieczne sekrety, audyt i mierzone wykonanie. Powiązania istniejące: 0.43 (OMAt), 0.44 (OAuth/OIDC/Vault), 0.07 i 1.51 (capabilities), 0.38–0.39 (WASI/Firecracker), 0.46/1.15/1.18/1.19 (Zenoh SHM/CRDT), 327/322 (Titans/pamięć), 328/326 (AST/SMT), 329 (sandbox i narzędzia). NIE renumerować istniejących 0.45–0.49; w PDF mają inne znaczenia. Nie akceptować symulacji, nieprawdziwych wskaźników pamięci między hostami, niezweryfikowanych <10us ani fikcyjnego formal proof. Moduł capability HMAC w src/sasos_capability.rs jest podłączony do bramki OMAt/Zenoh i kompozycji OAuth, ale wykonanie native/WASI/MicroVM pozostaje fail-closed; żaden wpis katalogu 555 nie jest automatycznie aktywowany. Operator-bound HTTPS przez VaultSecretBroker ma oddzielną, ściśle ograniczoną ścieżkę adapterów. **Kryterium zakończenia:** PASS dopiero po rzeczywistym procesowym E2E co najmniej jednego backendu, pozytywnych i negatywnych testach izolacji dwóch tenantów, odwołania, zawężania uprawnień, restartu, replay, timeoutu, weryfikacji efektu i metryk; każdy z 555 wpisów ma indywidualny dowód backendu przed zmianą CATALOG_ONLY. Eksperymenty Imandra/Titans/NICE/CRDT wymagają odrębnych sprawdzalnych artefaktów i benchmarków. Dowody CI i SHA, brak produkcyjnego GO bez odbioru operatora.. **Stan źródłowy:** `PLANNED`. | **DO ZROBIENIA** |
| **AB-CLOUD-CODE** | **Agents Builder Chmurowy Code** — wymagania pierwotne etapów 0.52. Web IDE + typed graph IR + Rust backend/Axum + Git integration + isolated build/execute + deployments + versioning **Kryterium zakończenia:** Browser Code IDE edits graph DSL, compiles/simulates typed agent, persists versioned project in tenant-scoped Cloud workspace; full build log. **Stan źródłowy:** `NOT_IMPLEMENTED`. | **DO ZROBIENIA** |
| **AB-CLOUD-NOCODE** | **Agents Builder Chmurowy NoCode** — wymagania pierwotne etapów 0.52. Browser visual drag/drop graph editor + templates, form nodes, visual tool/MCP grants and one-click preview/publish behind explicit approval **Kryterium zakończenia:** Browser NoCode canvas builds equivalent graph from nodes, round-trips graph JSON with compiled DSL for supported subset, and previews without source editing. **Stan źródłowy:** `NOT_IMPLEMENTED`. | **DO ZROBIENIA** |
| **AB-WINDOWS-CODE** | **Agents Builder Desktopowy Windows Code** — wymagania pierwotne etapów 0.53. Native Windows desktop Code IDE (Tauri v2) with local code editor, diagnostics, dependency runner, export to local agent bundle + offline mode **Kryterium zakończenia:** Actual Windows runner builds, installs and launches signed/verified executable; Code IDE opens agent project, compiles valid Rust/DSL, persists/reopens offline. **Stan źródłowy:** `NOT_IMPLEMENTED`. | **DO ZROBIENIA** |
| **AB-WINDOWS-NOCODE** | **Agents Builder Desktopowy Windows NoCode** — wymagania pierwotne etapów 0.53. Windows desktop NoCode designer, node library, visual connections/parameters, local preview, export + user-approved publishing **Kryterium zakończenia:** Actual Windows GUI smoke: NoCode drag/drop creates graph, save/reopen offline, convert supported graph to Code, publish artifact only with explicit approval. **Stan źródłowy:** `NOT_IMPLEMENTED`. | **DO ZROBIENIA** |
| **AB-MACOS-CODE** | **Agents Builder Desktopowy macOS Code** — wymagania pierwotne etapów 0.53. Native macOS desktop Code IDE (Tauri v2) including notarization-ready packaging, keychain policy, codesign verification and offline agent editing **Kryterium zakończenia:** Actual macOS runner builds/boots executable, validates bundle codesign entitlement metadata, edits and compiles Agent Builder Code project offline. **Stan źródłowy:** `NOT_IMPLEMENTED`. | **DO ZROBIENIA** |
| **AB-MACOS-NOCODE** | **Agents Builder Desktopowy macOS NoCode** — wymagania pierwotne etapów 0.53. macOS native NoCode canvas, typed forms and graph, local simulator and editable DSL export with reversible subset checks **Kryterium zakończenia:** Actual macOS GUI smoke saves/loads NoCode agent, connects nodes, previews, converts and checks typed IR roundtrip, controls permissions. **Stan źródłowy:** `NOT_IMPLEMENTED`. | **DO ZROBIENIA** |
| **AB-ANDROID-CODE** | **Agents Builder Android Code** — wymagania pierwotne etapów 0.54. Android Code editor (Kotlin/Compose + UniFFI Rust core), permission-aware on-device DSL edit/validation, mobile build through explicit external authorized compiler **Kryterium zakończenia:** Actual Android ARM64 emulator/device installs APK, creates/edits Code agent in bounded DSL, saves locally offline and resumes after app restart. **Stan źródłowy:** `NOT_IMPLEMENTED`. | **DO ZROBIENIA** |
| **AB-ANDROID-NOCODE** | **Agents Builder Android NoCode** — wymagania pierwotne etapów 0.54. Android touch NoCode designer with offline graph CRUD, gestures, templates, safe-preview, signed import/export and user-mediated permission grant **Kryterium zakończenia:** Android instrumentation creates visual NoCode graph, validates typed edges and safe preview, exports/imports offline, survives process death. **Stan źródłowy:** `NOT_IMPLEMENTED`. | **DO ZROBIENIA** |
| **AB-IOS-CODE** | **Agents Builder iOS Code** — wymagania pierwotne etapów 0.54. iOS Code editor (SwiftUI + UniFFI Rust core), edit and static-validate bounded agent DSL, code build/export via authorized remote toolchain, Sandbox constraints **Kryterium zakończenia:** Actual Xcode iOS simulator/device run creates/edits Code agent under app Sandbox, validates typed agent DSL, restores local project after restart. **Stan źródłowy:** `NOT_IMPLEMENTED`. | **DO ZROBIENIA** |
| **AB-IOS-NOCODE** | **Agents Builder iOS NoCode** — wymagania pierwotne etapów 0.54. iOS SwiftUI touch NoCode agent builder with offline graph designer, export/import via Document Picker, user-controlled tool entitlements **Kryterium zakończenia:** Actual iOS simulator UI builds multi-node NoCode workflow, round-trips supported IR with Code view, persists offline and respects app life-cycle. **Stan źródłowy:** `NOT_IMPLEMENTED`. | **DO ZROBIENIA** |


### Reguła aktualizacji na zakończenie każdego etapu

Przy **każdym** zakończonym etapie aktualizować w tym samym commicie: odpowiedni status pozycji powyżej, `ROADMAP.md`, jego identyczną kopię `docs/ROADMAP_FULL_ECOSYSTEM_2026.md`, maszynowy `docs/roadmap/canonical_v11_inventory.json`, `docs/roadmap/STAGE_CHANGELOG.md`, wyniki pozytywnych i negatywnych testów, commit i zakończony run GitHub Actions. Wszelkie nowe wymagania dostają wyłącznie **kolejny wolny identyfikator** (teraz 334), a odzyskane historyczne wymagania zachowują oryginalny. Lista 1–408 jest deterministycznie odtwarzana z manifestów, żeby zmiany statusów nie były ręcznie pomijane.


### Uzgodnienie źródeł PDF 01–09 z jedną Roadmapą

Rejestr źródłowy (nie oznacza zakończenia implementacji):

| PDF | Zakres zachowany w roadmapie | Etapy / stan |
|---|---|---|
| 01 | Wymagania infrastruktury, agentów i bezpieczeństwa | 0.39, 0.43–0.44, 0.48–0.49, 0.55, 0.63; wymagają testów |
| 02 | Rust/Zenoh, bezpieczeństwo transportu i integracje | 0.39, 0.43–0.49, 0.55, 0.63, 0.91, 1.92; wymagają testów |
| 03 | Architektura Rust, SHM/CRDT, MCP, Firecracker/Aya, PQC | 0.39, 0.43–0.49, 0.55, 0.63, 0.91; wymagają testów |
| 04 | Drugi egzemplarz materiału PDF 03 | Zachowano osobne pochodzenie, bez dublowania funkcji |
| 05 | JARVIS OMEGA INFINITY, fazy M0–M5 | 0.39, 0.41, 0.43–0.49, 0.63, 0.91, 1.92 |
| 06 | Drugi egzemplarz materiału PDF 05 | Zachowano osobne pochodzenie, bez dublowania funkcji |
| 07 | Rozszerzenia architektury, M6 i ograniczenia wdrożeniowe | Historyczne wpisy w ROADMAP; szczegółowa semantyczna zgodność wymaga dalszego audytu |
| 08 | M7 wetware, M8 holograficzny stan, M9 kryptografia topologiczna | 0.91, 0.46–0.47, 0.45/0.63; badawcze, nie zaimplementowane |
| 09 | Symulacje społeczne, ToM, DSPy, Agent-Zero/Hermes, AI Scientist, AutoGPT/BabyAGI, GodMode, OpenHands, SWE-bench | **Nowe etapy 208–215** (PLANNED), powiązane z 1.98–2.06 |

**Reguła źródłowa:** pliki PDF 01–09 są uwzględnione jako źródła wymagań; nie jest to certyfikacja kompletności semantycznej wszystkich stron. Nie przenosić tej tabeli ani głównej Roadmapy poza początek README. Dalsze odkryte luki dodawać bez usuwania historycznych wpisów.

<!-- OCAF_README_ROADMAP_END -->

> **📌 [PEŁNA ROADMAPA OCAF OD SAMEGO POCZĄTKU — ROADMAP.md](ROADMAP.md)** · [Etapy 0.00–1.94 i statusy wykonania](ROADMAP.md#statystyka-wykonania-i-uczciwe-oznaczenia) · [10 pierwotnych Agents Builder Code / NoCode](docs/AGENTS_BUILDER_CODE_NOCODE_5_PLATFORMS.md) · [inwentaryzacja braków](docs/INVENTORY_LOST_REQUIREMENTS_2026-10-15.md) · [dziennik zmian po każdym etapie](docs/roadmap/STAGE_CHANGELOG.md)  
> **Jedyna kanoniczna Roadmapa v11 jest przypięta na samym początku README oraz w katalogu głównym.** 195 etapów, 64 podetapy; nowo odkryte elementy dopisywane tylko kolejnymi numerami, stare wymagania przywracane pod pierwotnymi etapami. 

# OCAF Runtime — Rust / Tokio / Zenoh

> **ROADMAPA KANONICZNA v10 — OD POCZĄTKU PROJEKTU**  
> [Otwórz pełną Roadmapę](docs/ROADMAP_FULL_ECOSYSTEM_2026.md): **195 numerowanych etapów głównych 0.00–1.94 + 64 zidentyfikowane podetapy = 259 pozycji wykonawczych**, z odniesieniami do **33 repozytoriów referencyjnych**. [Pełna ewidencja maszynowa](docs/roadmap/canonical_v10_inventory.json); [oryginalny zapis v9 bez żadnych strat](docs/archive/ROADMAP_FULL_ECOSYSTEM_v9_2026-10-15.md). Nieodtworzone 0.00–0.10 i historyczne tytuły 0.11–0.35 jawnie oznaczone, zamiast dopisywać fikcyjne wymagania. Techniczne zamknięcie Firecracker WASI, niezależnych hostów PostgreSQL HA i długich testów odporności nadal wymaga dowodów.

## Zamknięcie kamienia milowego 0.38 — COMPLETE (2026-10-15)

**STAGE 0.38 = COMPLETE  (status prac w roadmapie).**
Wszystkie dalsze zadania rozwojowe przeniesiono do 0.39 lub odrębnych bramek odbiorowych.
Zakończenie kamienia milowego.
**Odbiór administracyjny operatora: COMPLETE. Techniczne A01–A12 oraz niezależny audyt nie mają kompletu rzeczywistych dowodów. `production_approved=false`, `production_deployed=false`; etap 0.38 zamknięto zarządczo, nie jako certyfikat produkcyjny.**
Zapisy historyczne `PARTIAL` w poniższych sekcjach odnoszą się do poprzednich
dat; nadrzędny aktualny status zarządczy brzmi `COMPLETE`.
Szczegóły: [decyzja operatora](docs/STAGE038_OPERATOR_DIRECTIVE_2026-10-15.md)
i [rejestr dowodów](docs/STAGE_038_8_9_ACCEPTANCE.md).

**Status: experimental PostgreSQL-backed active/passive coordinator, signed Zenoh RPC and durable distributed jobs. Production-certified HA is not established.**

OCAF provides an async local tool registry and execution engine, plus a
**real Zenoh query/reply protocol** for pairwise authenticated agent calls.
The protocol uses HMAC-SHA256, a per-peer tool allowlist, bounded frames,
request expiration and either an in-memory or durable SQLite replay journal. Remote execution is
deliberately limited to **free, enabled tools without special permissions**.

## Capabilities

| Component | Status |
| --- | --- |
| Rust/Tokio in-process tool execution | Implemented |
| Manifest registry and JSON Schema input/output validation | Implemented |
| Local timeouts, atomic budget escrow and accounting | Implemented |
| Versioned Zenoh remote query/reply | Implemented |
| Pairwise HMAC-SHA256 authenticated request/response | Implemented |
| Exact tool allowlist and signed tool discovery | Implemented |
| SQLite WAL replay journal / expiration / bounded concurrency | Implemented; mandatory in RPC server CLI |
| Authenticated peer-specific RPC rate limit | Default 64/s; tunable through PeerPolicy |
| Two-session Zenoh TCP integration test | Automated test |
| TLS/mTLS transport policy | Fail-closed CLI preflight; requires TLS-only/mTLS settings and certificate files |
| Durable admission (at-most-once within TTL) | Implemented with SQLite; not exactly-once execution |
| SQLite job queue, worker lease/fencing, restart recovery | Implemented for one host |
| Authenticated agent membership, 15-second leases, capability routing | Implemented (single-host coordinator) |
| Signed remote job submit/pull/finish and owner-only results | Implemented; single coordinator |
| On-host coordinator active/passive leadership | Implemented with exclusive OS file lock and standby takeover |
| PostgreSQL cross-host advisory leadership | Integrated into opt-in active/passive coordinator service |
| Shared PostgreSQL remote job queue | Experimental, role-restricted and transactional; opt-in only |
| Shared PostgreSQL replay and agent directory | Implemented for opt-in PostgreSQL cluster mode |
| Cross-host active/passive process promotion | Experimental; requires a shared PostgreSQL primary and external stable endpoint |
| Built-in PostgreSQL replication / Raft consensus | Not implemented |
| L402 Lightning payments and cryptographic payment validation | Not implemented |
| Untrusted executable tool sandbox and production rate limiting | Not implemented |

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --no-default-features -- -D warnings
cargo test --no-default-features
cargo test
cargo run --bin ocaf_runtime
```

The library builds without Zenoh using `--no-default-features`. The
`ocaf-rpc-node` binary requires the Zenoh feature.

## Local two-process RPC demo

**Security warning:** The following files use plaintext TCP bound to
`127.0.0.1`. This is a *local-only test*, not secure remote deployment.

Provision a shared 32-byte key to each process, for example:

```bash
export OCAF_SHARED_KEY_B64="$(openssl rand -base64 32)"
```

Copy that environment variable securely into both local shell sessions.
Do not check the key into source control or expose it in screenshots.

**Server terminal:**

```bash
export OCAF_ROLE=server
export OCAF_NODE_ID=agent-b
export OCAF_PEER_ID=agent-a
export OCAF_ZENOH_CONFIG=examples/insecure-local/server.json5
export OCAF_ALLOW_INSECURE_LOOPBACK=1 # Local-only unsafe demo
export OCAF_REPLAY_DB_PATH="$(pwd)/ocaf-replay.sqlite"
cargo run --bin ocaf-rpc-node
```

**Client terminal (with the same shared key):**

```bash
export OCAF_ROLE=client
export OCAF_NODE_ID=agent-a
export OCAF_PEER_ID=agent-b
export OCAF_ZENOH_CONFIG=examples/insecure-local/client.json5
export OCAF_ALLOW_INSECURE_LOOPBACK=1 # Local-only unsafe demo
export OCAF_ACTION=discover
cargo run --bin ocaf-rpc-node
export OCAF_ACTION=call
export OCAF_MESSAGE="authenticated request over Zenoh"
cargo run --bin ocaf-rpc-node
```

The server exports only `demo_echo`; it executes through the real
`OcafRuntime` after verifying the caller's signature, ID, TTL, replay
status, ACL and manifest safety. Replies are independently signed.
Calling unknown/paid/restricted tools is denied.

## Deploying across machines

Use a reviewed Zenoh JSON5 configuration via `OCAF_ZENOH_CONFIG` for
both peers, with reachability and an authenticated encrypted transport
(e.g. configured TLS/mTLS). The **presence of a config file is NOT proof**
of TLS or peer identity. Confirm effective Zenoh transport behavior and
certificates in the deployment environment. The in-process HMAC
authenticator offers integrity and peer-key authentication, **not**
confidentiality.

Both RPC and coordinator CLIs **require** `OCAF_ZENOH_CONFIG` and run a
fail-closed transport policy validator. Secure mode requires explicit
TLS-only endpoints/protocol whitelist and mTLS certificate settings.
Plaintext TCP is permitted only with the explicit development flag
`OCAF_ALLOW_INSECURE_LOOPBACK=1` and literal loopback endpoints;
non-loopback TCP is rejected even with that flag. These checks do not
replace actual Zenoh certificate handshake verification. See
[docs/HA.md](docs/HA.md) and [examples/secure-mtls](examples/secure-mtls).


## Wire contract

- Fixed keyspace: `ocaf/v1/nodes/{target-node}/rpc`.
- `RpcRequest` includes version, UUID correlation ID, sender,
  recipient, issuance time, short TTL and signed discovery, execution,
  coordinator and job control actions.
- The body bytes are base64-encoded inside a JSON envelope and signed
  using per-peer HMAC-SHA256. Both sides verify the signature and ID
  binding. No arbitrary tool code is transferred.
- The library can use volatile replay protection, but the RPC server CLI
  requires an absolute `OCAF_REPLAY_DB_PATH` and uses durable SQLite WAL.
  It commits an admission claim before execution; repeating a signed request
  ID after restart is rejected within its TTL window. It is **not exactly-once
  execution**. If the server crashes after admission but before a tool finishes,
  the outcome may be unknown and retries with the same ID are denied.
  SQLite is intended for a single host or replicas sharing the same on-disk
  filesystem; **it is not a cross-host consensus or distributed database**.
- Per-peer authenticated RPC quota defaults to 64 requests/s per gateway;
  `PeerPolicy::with_rate_limit` can set an explicit 1–10,000 requests/s.
  Quotas are fixed-window and per-process, not globally synchronized.
- Size limits: 64 KiB frame, 32 KiB body; maximum request TTL: 30 s.
- Do not send personal/private data over the demo's plaintext transport.

See [docs/SECURITY.md](docs/SECURITY.md) for threat model and limitations.

## Durable local jobs (v0.6)

The `JobStore` module implements persistent FIFO work admission on SQLite
with UUID-based idempotent submission, atomic claims, lease fencing,
result persistence and retry policies. `JobWorker` integrates claims with
the existing runtime and requires an explicit tool allowlist; paid and
permissioned tools remain blocked. Expired safe-to-retry jobs can be
reclaimed; otherwise they become `uncertain` pending manual investigation.

**A lease does not provide exactly-once side effects.** Mark jobs
`retry_safe` only when operations are actually idempotent. This is
single-host orchestration, not cluster consensus. As of v0.8, a separate
**signed, role-restricted remote job API** uses this SQLite store.

See [docs/JOBS.md](docs/JOBS.md) for API examples, persistence guarantees,
operating requirements and remaining architectural work.

## Authenticated capability coordinator (v0.7)

The standalone `ocaf-coordinator` service accepts signed agent heartbeats
and capability lookups over Zenoh. Each agent advertises only tools explicitly
authorized in its administrator-configured HMAC peer policy. The persistent
SQLite directory expires membership after 15 seconds without a heartbeat
and rotates lookups fairly among currently advertised nodes.

The demo RPC server can automatically renew its membership every five
seconds when `OCAF_COORDINATOR_NODE_ID` and `OCAF_COORDINATOR_KEY_B64`
are provisioned. HMAC does not encrypt traffic; the operator must separately
enforce verified Zenoh TLS/mTLS for non-loopback deployments.

See [docs/COORDINATOR.md](docs/COORDINATOR.md) for the separate daemon,
peer provisioning, acceptance tests, and important non-production limitations.

## Distributed job orchestration (v0.8)

The coordinator now accepts signed `SubmitJob` from explicitly designated
submitters, assigns queued work via `ClaimJob` to allowlisted remote workers,
and accepts fenced `FinishJob` reports. Only the original authenticated
submitter can retrieve its result through `GetJob`. Distinct submitter and
worker permissions are configured in `OCAF_PEERS_FILE`.

The `ocaf-rpc-node` CLI now supports `OCAF_ROLE=worker` (polling executor)
and `OCAF_ACTION=submit` / `OCAF_ACTION=status` in client mode. The
coordinator requires `OCAF_JOBS_DB_PATH` in addition to the previous two
SQLite paths.

Remote jobs are limited to one attempt, are never retried automatically
after ambiguous failures, and may enter the terminal `uncertain` state.
HMAC authenticates peers but does not encrypt job arguments or results.
There is **no replicated scheduler, enforced TLS/mTLS or exactly-once side
effect guarantee**.

See [docs/DISTRIBUTED_JOBS.md](docs/DISTRIBUTED_JOBS.md) for the complete
protocol, role ACLs, local demo and security boundaries.

## Local active/passive coordinator and transport preflight (v0.9)

The coordinator now requires an absolute `OCAF_COORDINATOR_LOCK_PATH`
in the same local directory as its SQLite job database. It refuses to
serve without exclusive ownership of the process-held OS file lock.
A second coordinator started with `OCAF_COORDINATOR_MODE=standby`
waits and promotes after the original leader releases the lock, including
abnormal process exit. Both instances must share the **same local host,
local disk, endpoint, databases and node identity**; external process
supervision is required. This is not HA for a failed machine or a
replicated multi-server cluster.

See [docs/HA.md](docs/HA.md) for configuration, security acceptance tests,
limits, and the next cross-host architecture milestones.

## PostgreSQL active/passive coordinator (v0.11)

Set `OCAF_CLUSTER_MODE=postgres` to enable the standalone coordinator's
PostgreSQL-backed path. The gateway stores its replay ledger, capability
directory and remote-job queue in a **single shared PostgreSQL primary**,
while a dedicated session advisory lock elects one active coordinator
across hosts. The standby does not open its Zenoh listener until the
database lock is available. A per-request leadership check and periodic
monitor terminate the service on detected loss of leadership.

Local SQLite mode remains available unchanged. The PostgreSQL cluster
mode requires `OCAF_CLUSTER_ID`, `OCAF_CLUSTER_PG_DSN`, existing secure
Zenoh and peer settings, and `OCAF_COORDINATOR_MODE=active` or `standby`.
Neither `OCAF_COORDINATOR_LOCK_PATH` nor local SQLite paths are required
in this mode. There is **no automatic migration** of existing SQLite
data.

Real PostgreSQL integration tests check concurrent leadership, atomic
job claims, shared replay rejection, capability expiry, signed gateway
routing, loss-of-leadership denial and real Zenoh-to-PostgreSQL job
execution.

**Production gaps:** PostgreSQL HA/failover and replication, a stable
network endpoint for promoted nodes, actual multi-host outage
injection, operational PKI testing, strong external-effect fencing,
certificate rotation and independent security review. The runtime
does not supply Raft or eliminate the possibility of
`uncertain` external side effects.

See [docs/POSTGRES_CLUSTER.md](docs/POSTGRES_CLUSTER.md) for deployment
configuration, acceptance tests and threat boundaries.

## Transactional audit and crash fault injection (v0.12)

The PostgreSQL job backend now records metadata-only submission, claim,
completion and crash-recovery events in the same transaction as the
associated job-state transition. Signed `GetJobAudit { id, limit }` allows
an authenticated original submitter (not a worker or another submitter)
to inspect at most 100 audit entries. The log does **not** contain tool
inputs, outputs, credentials or raw lease tokens. Legacy SQLite mode
rejects this audit operation instead of inventing audit history.

PostgreSQL fault-injection tests terminate the active leader's actual
database backend using `pg_terminate_backend`, verify standby
promotion, deny requests through the previous leader and ensure the
submitted task and replay-admission record survive the promotion.
A separate test verifies stale completion cannot turn an expired,
`uncertain` job into a successful one.

These are PostgreSQL **session-failure** experiments run in CI; they do
not establish physical cross-host failover, distributed partition safety,
production mTLS validation or exactly-once external effects.
See [docs/RESILIENCE.md](docs/RESILIENCE.md).

## Cluster observability and operator controls (v0.13)

The new **read-only operator CLI**, `ocaf-cluster-health`, collects
PostgreSQL-clock snapshots of advisory leader sessions, live/expired
agent heartbeat leases, durable jobs by status, and expired running
leases. It prints **JSON reports or label-free Prometheus-style metrics**
to stdout and returns 0 (healthy), 1 (warning), or 2 (critical).
`--no-fail` preserves collection success exit codes for external
exporters. Alert thresholds are configurable through environment
variables. `--node <ID>` reports a single agent's lease status only
in protected JSON output.

The CLI does not expose HTTP and does not publish signing keys,
job arguments, results, tool ACLs, or lease tokens. It requires a
provisioned cluster database and operator-approved PostgreSQL
credentials. The cluster ID selects the **advisory leader key**, not
a tenant namespace; use an isolated PostgreSQL database or schema
for each trust domain.

This is **operational visibility, not automatic repair**. Heartbeat
leases do not prove that workers are reachable; PostgreSQL advisory
locks do not prove client routing works; and external notifications,
database replication and host/network-partition chaos testing remain
operator responsibilities.

See [docs/OBSERVABILITY.md](docs/OBSERVABILITY.md) for usage, thresholds,
exit codes, safeguards and CI coverage.

## Stable-endpoint reconnect and deployment hardening (v0.14)

The `ocaf-rpc-node` **client** role now uses a reconnecting
authenticated Zenoh transport. It recreates a vetted Zenoh session
with bounded exponential backoff after transport failure and permits
automatic retry **only** for read-only discovery, owner-only job status
and audit, or UUID-idempotent job submission. Remote tool `Execute`,
worker `ClaimJob`, `FinishJob` and `Heartbeat` are **never**
automatically retried, because a lost reply may conceal side effects
or an active lease. No retries are unbounded.

`OCAF_RECONNECT_ATTEMPTS` controls the attempt count (1–8, default 4).
`OCAF_ACTION=submit` now **requires a stable, explicit
`OCAF_JOB_ID`**, reused for every attempt and invocation. Do not
generate a new job UUID when a previous response is ambiguous.

The coordinator validates its PostgreSQL leader session **again just
before sending a signed reply**. This reduces stale-leader
acknowledgments but cannot undo effects that happened during an
in-flight request or replace external-system fencing.

New hardened systemd coordinator and health units are in
[deploy/systemd](deploy/systemd). They are templates requiring an
operator-provisioned database, secure Zenoh/mTLS PKI, secret files,
privileged endpoint routing and supervision. Tests exercise bounded
retry semantics and a real Zenoh server starting after the client.

See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) for security assumptions,
commands and remaining machine-level verification work.

## Cryptographic Zenoh mTLS integration gates (v0.15)

The full-feature integration suite now constructs temporary private
certification authorities and certificates with OpenSSL, validates the
operator's TLS-only configuration with the exact CLI preflight, and
executes **real signed Zenoh RPC over mutually authenticated TLS**.
It rejects clients presenting certificates issued by a different CA
and clients trusting an unrelated server CA. Test credentials are
generated at runtime, **never committed** as long-lived secrets.

This checks actual TLS handshakes rather than configuration shape
alone. However, CI still runs on one loopback host, not on physically
separate machines; operational PKI trust, certificate rotation,
revocation, DNS failover, live PostgreSQL failover and multi-host
network partition scenarios remain unverified.

See [docs/MTLS_VERIFICATION.md](docs/MTLS_VERIFICATION.md) and
[tests/zenoh_mtls_e2e.rs](tests/zenoh_mtls_e2e.rs).

## PostgreSQL-authoritative job lease clock (v0.16)

PostgreSQL queue claims, expiry recovery and fenced job completion now use
the **database server clock** within SQL, not the coordinator host clock.
This prevents skewed active/standby coordinators from changing lease
expiration decisions. The existing `now_ms` API argument remains for
compatibility but is ignored in the PostgreSQL backend (not SQLite).
Tests cover forward/backward host-clock skew and database-side expiry.

This does not replace PostgreSQL HA, operational clock synchronization,
host-level fault injection or external-system fencing.

See [docs/LEASE_CLOCK.md](docs/LEASE_CLOCK.md).

## Durable leadership fencing epochs (v0.17)

The PostgreSQL active/passive election now records a **strictly increasing
epoch per cluster** in the shared primary. Every RPC admission and signed
reply leadership verification checks both the active backend's *actual*
PostgreSQL advisory lock and the durable generation, not merely whether
its TCP session answers `SELECT 1`.

Acquisition and release are serialized per coordinator to prevent stacked
re-entrant PostgreSQL advisory locks. Ambiguous lock/epoch transitions
poison the connection and fail closed. `PgLeader::fencing_epoch()` exposes
the checked generation to explicitly cooperating downstream systems.

**This is coordinator fencing, not external-effect fencing.** A stale
agent already executing arbitrary external tools can still cause side
effects unless that external system independently enforces the epoch.
A single PostgreSQL primary is still assumed; physical split-brain
partitions and database failover are not certified.

See [docs/FENCING_EPOCH.md](docs/FENCING_EPOCH.md).

## Remote-worker epoch pre-execution checks (v0.18)

A PostgreSQL cluster leader's checked, monotonic fencing generation is
now embedded in signed job leases, persisted in the queue, and checked
before worker execution via signed `ValidateLease`. The coordinator
also holds the epoch row lock throughout claim and completion state
transitions. Expired and previous-generation completions are rejected.

Local SQLite and standalone PostgreSQL retain generation zero and their
existing behavior. **Update clustered workers and coordinators together**:
cluster lease JSON now includes `leader_epoch`, requiring compatible
worker software. This is a point-in-time coordinator authorization,
not atomic fencing by arbitrary external services.

See [docs/WORKER_FENCING.md](docs/WORKER_FENCING.md).

## Transactional PostgreSQL effect sink (v0.19)

The signed worker RPC now offers an **explicit, opt-in PostgreSQL-local**
`ApplyFencedEffect` operation, separate from arbitrary external tools.
Inside a single PostgreSQL transaction, the coordinator locks the
current leadership generation and the exact running job lease before
inserting an immutable, one-per-job JSON effect. Duplicate identical
attempts return the original commit timestamp; altered payloads or
stale/expired leases are rejected. The sink stores no HMAC keys or
raw lease tokens and does not expose full payloads in its receipt.

This **demonstrates and enforces fencing for writes to this one database
table only**. It does not authorize or atomically fence external HTTP,
email, payments, filesystem access or arbitrary model/tool execution.
Those systems each require an independent transactional fencing adapter.
The experimental runtime creates the new table on startup; migrations
and least-privilege roles still need production hardening.

See [docs/FENCED_EFFECTS.md](docs/FENCED_EFFECTS.md).

## Fail-closed cluster execution & effect privileges (v0.20)

Cluster-mode coordinators now **reject direct RPC Execute**, even for
nominally free, allowlisted tools: a tool manifest cannot prove
that its executor has no external side effects. Standalone RPC
execution remains available. Transactional `ApplyFencedEffect` is now
**default deny** and requires a separate per-peer
`write_effects: true` permission *in addition to* `worker_jobs: true`.
Existing worker configurations cannot write to the sink by accident.

Cluster remote workers also refuse to run any ordinary tool under a
positive leader generation unless its name is explicitly included in
`OCAF_CLUSTER_PURE_TOOLS`, a comma-separated allowlist vetted by the
operator as actually side-effect-free. An unset variable denies all
such clustered tool execution; trusted demo `demo_echo` can be enabled
via `OCAF_CLUSTER_PURE_TOOLS=demo_echo`. An incorrectly classified
tool can still cause unfenced effects; this is a safety boundary, not
a general-purpose external side-effect protocol.

See [docs/EFFECT_BOUNDARY.md](docs/EFFECT_BOUNDARY.md).

## Independent authenticated effect receiver (v0.21)

New standalone `ocaf-effect-sink` executable owns an **independent
SQLite event journal** and a separate signed Zenoh RPC destination.
Trusted controllers publish short-lived, monotonic leadership epochs;
worker writes require a separately controller-signed, job-and-payload-
bound permit, the current epoch, a valid local activation window and
durable nonce/idempotency checks. This supports independent delivery
and acknowledgement of *journal events*; it does **not** atomically
fence arbitrary third-party APIs or prove externally visible
side effects have completed.

See [docs/INDEPENDENT_SINK.md](docs/INDEPENDENT_SINK.md) for
configuration, identity/key isolation, security guarantees, CI
coverage and current split-brain limitations.

## Cluster-scoped verified sink permits (v0.22)

Sink protocol v2 requires an explicit `OCAF_SINK_CLUSTER_ID`, and
receiver permits now cryptographically bind the target cluster, specific
PostgreSQL job tool, attempt and SHA-256 fingerprint of its unpredictable
lease token. The `ZenohSinkClient::issue_permit_for_lease` safe issuance
path checks *the actual current PostgreSQL lease*, the attached leader
cluster ID, and a maximum 15-second permit window before signing.
Legacy unscoped v1 permits are rejected. See
[docs/SINK_IDENTITY.md](docs/SINK_IDENTITY.md) for migration and caveats.

## Automatic PostgreSQL HA failover and namespace isolation (v0.23)

New `OCAF_COORDINATOR_MODE=ha` reconnects and re-elects a verified
PostgreSQL leader after a service interruption; standbys remain unable
to serve while un-elected. On promotion, previous-epoch worker leases
are **immediately quarantined as uncertain**, never auto-retried.
A periodic fenced sweep reconciles expired remote leases independently
of worker polling. The shared job queue now scopes operations by
`cluster_id`, preventing unrelated clusters on one database from
claiming or recovering each other's jobs. Schema migration for
pre-0.23 leader-managed records requires explicit operator review.

Deployment, migration, tests and physical HA limitations:
[docs/HA_FAILOVER.md](docs/HA_FAILOVER.md).

## Network and infrastructure chaos scenarios (v0.24)

The new deterministic loopback-only TCP fault-injection proxy tests genuine
PostgreSQL socket partitions and deliberately unresponsive handshakes. A
separate real-Zenoh transport test simulates coordinator host disappearance,
PostgreSQL leader-session termination and reconnect of a freshly signed
client to the replacement coordinator. GitHub Actions now has explicit
chaos stages in addition to complete core and Zenoh regression tests.

These are **separate sockets/process-like service sessions on one CI host**,
not multi-host networking or a replicated-PostgreSQL partition test. No
production multi-host or dual-primary availability guarantees follow from
these checks. See [docs/CHAOS_ENGINEERING.md](docs/CHAOS_ENGINEERING.md)
for scenario details, expected invariants and a physical-host acceptance
matrix to complete before production approval.

## Production operations: scoped health and secure configuration (v0.25)

`ocaf-cluster-health` is a **read-only PostgreSQL** health command. It
outputs a single JSON report containing leader counts, alerts and bounded
aggregates; no job payloads, signing secrets, identities or raw DSN are
returned. A missing leader or an unavailable database is **not reported as
healthy** (nonzero exit for critical/unavailable). Job counters are
scoped to the configured `OCAF_CLUSTER_ID`. Note that v1 agent
membership currently remains **database-global**, including per-node
lookups; do not regard its agent counts as tenant-isolated.

For the coordinator and health CLI, `OCAF_CLUSTER_PG_DSN_FILE`
is accepted as an alternative to the original `OCAF_CLUSTER_PG_DSN`.
The private file must be an absolute, regular, service-user-owned
mode-0600 file on Unix; file and environment credential sources cannot
be enabled simultaneously. The coordinator's peer JSON file now uses
the same secure descriptor reader, rejecting symlink swaps and
over-permissive files. This does not implement live credential rotation.

Deployment templates, health timer, alarm thresholds, migration
preflight, rollback and remaining risks are in
[docs/PRODUCTION_OPERATIONS.md](docs/PRODUCTION_OPERATIONS.md).

## Final development acceptance (v0.26)

The v0.26 security regression suite validates isolated PostgreSQL
replay journals and agent capability directories with **composite cluster/node identity**,
namespace-specific routing cursors and transactional fencing of
directory writes. A cluster cannot consume another cluster's replay-nonce quota
or read another cluster's capability
memberships or consume its round-robin state merely because both use
the same PostgreSQL primary. The read-only cluster health probe now
counts and looks up agents only within the selected namespace.

The v1 membership tables are **not automatically imported** into the v2
cluster-aware tables because their original cluster ownership cannot be
proved from the rows. On upgrade, agents must re-register via signed
heartbeats under an elected leader or operators must perform a
validated offline migration. Operators must retain the old tables and
backups until the change is verified. See
[docs/FINAL_ACCEPTANCE.md](docs/FINAL_ACCEPTANCE.md) for the tested
claims, migration, security limitations and **conditional GO/NO-GO**
criteria. A green GitHub Actions run is not physical multi-host
production certification.

## Production work still required

1. Certificate-backed TLS/mTLS handshakes on separate deployed hosts, plus certificate rotation and revocation.
2. PostgreSQL HA replication and external-effect fencing; real **cross-host** fault/partition certification.
3. Signed plugin supply chain, sandbox and capability isolation.
4. Secret manager integration, peer/key rotation, revocation and quotas.
5. Distributed traces, whole-host/partition fault injection, property tests and benchmarks.
6. Real L402 payment verification **only after** independent security review.

There are **no substantiated sub-millisecond latency claims** for this
prototype. Tests prove certain behavior, not all adversarial or
production network conditions.

## License

The crate declares MIT licensing in `Cargo.toml`. License/redistribution
status should be reviewed before public release.

## Kontrola gotowości wydania (v0.27)

Skrypt `scripts/release_readiness.py` sprawdza obecność i strukturę pliku
`Cargo.lock` oraz kompletność, integralność SHA-256 i aktualność
udokumentowanych dowodów operacyjnych. Niespełnione wymagania oznacza
`NO_GO`. Zgodność formalna artefaktów nie stanowi certyfikacji
bezpieczeństwa ani gotowości produkcyjnej. `Cargo.lock` znajduje się
już w `main`, ale nadal brakuje niezależnie zweryfikowanych dowodów
testów infrastrukturalnych. Produkcyjne zatwierdzenie pozostaje zablokowane.

Weryfikacja testów: `python3 -m unittest discover -s scripts/tests -v`.
Pełne wymagania i sposób użycia: [docs/RELEASE_READINESS.md](docs/RELEASE_READINESS.md).

## Łańcuch dostaw i potwierdzanie pochodzenia (priorytet 0.28)

`Cargo.lock` jest teraz śledzony w `main`, a CI sprawdza zapisany graf zależności
za pomocą `--locked`. Odrębny workflow kontroluje źródła pakietów,
advisories RustSec, licencje i integralność wpisów rejestru.
Po przejściu kontroli generuje artefakty Linux x86_64, CycloneDX SBOM
oraz żąda atestacji pochodzenia kompilacji i SBOM. Ich istnienie i weryfikacja
wymagają **zakończonego sukcesem** konkretnego przebiegu GitHub Actions.
Kontrole nie są certyfikacją produkcyjną.

**Historia:** zestaw domyślnych funkcji Zenoh został ograniczony do TCP/TLS,
co usunęło z `Cargo.lock` zależność `rsa`. Kolejne ostrzeżenia RustSec
wykryte na tym etapie zostały usunięte w priorytecie 0.30. Szczegóły:
[docs/SUPPLY_CHAIN_FINDINGS.md](docs/SUPPLY_CHAIN_FINDINGS.md).

Szczegóły: [docs/SUPPLY_CHAIN.md](docs/SUPPLY_CHAIN.md).

## Ograniczenia ryzyka transportu Zenoh (priorytet 0.29)

Konfiguracja Zenoh odrzuca włączenie kompresji zarówno dla transportu
unicast, jak i multicast. Oddzielne testy i kontrola rozwiązanego grafu
funkcji `cargo metadata --locked --all-features` ograniczają dopuszczone
funkcje do transportów TCP/TLS bez kompresji i dodatkowego mechanizmu
uwierzytelniania zależnego od RSA. W priorytecie 0.29 była to
tymczasowa kontrola ograniczająca ryzyko, **bez aktualizacji**
bibliotek. Niebezpieczne zależności usunięto w następnym etapie 0.30.

Szczegóły: [docs/SUPPLY_CHAIN_FINDINGS.md](docs/SUPPLY_CHAIN_FINDINGS.md).

## Migracja zależności Zenoh i audyt RustSec (priorytet 0.30)

W `main` wprowadzono pięć ściśle wyznaczonych backportów z kodu
`zenoh 1.10.1` pozyskanego z weryfikowanych przez Cargo archiwów.
Zaktualizowano `lz4_flex` do `0.11.6`, zastąpiono `paste` przez
`pastey 0.2.3`, usunięto `rustls-pemfile` na rzecz parsera
`rustls-pki-types` oraz wyłączono interpretację YAML (również w
plikach dołączanych), usuwając `serde_yaml` i `unsafe-libyaml`.
Własne pliki JSON5 oraz TLS/mTLS nadal podlegają testom integracyjnym.
Z Cargo.lock usunięto pakiety powiązane ze wszystkimi czterema
pierwotnymi ostrzeżeniami RustSec.

Automatyczny proces migracji przeszedł testy Rust, Clippy i
`cargo-deny check --all-features` **bez wyciszania ostrzeżeń**
(GitHub Actions: `38371010671`). Każdy kolejny commit ma osobny
zestaw weryfikacji CI; powodzenie backportu nie oznacza jeszcze
certyfikowanego wydania. Utrzymanie lokalnych poprawek oraz niezależny
przegląd kodu pozostają obowiązkowe.

Dokumentacja: [docs/ZENOH_MIGRATION_030.md](docs/ZENOH_MIGRATION_030.md),
[docs/SUPPLY_CHAIN_FINDINGS.md](docs/SUPPLY_CHAIN_FINDINGS.md).
Stan produkcyjny pozostaje **NO-GO** z powodów infrastrukturalnych,
mimo usunięcia wymienionych ostrzeżeń zależności.

## Priorytet 0.31 — naprawa artefaktów CycloneDX i atestacji

Naprawiono błąd generowania pliku SBOM: wersja `cargo-cyclonedx 0.5.9`
automatycznie dopisuje rozszerzenie `.json` do nazwy ustawionej przez
`--override-filename`. Użycie `bom.json` powodowało wygenerowanie
`bom.json.json`; workflow oczekiwał `bom.json`. Zmieniono
nazwę bazową na `bom`, dodano test regresyjny nazwy wyjściowej,
walidację struktury SBOM CycloneDX 1.5 oraz uzupełniono dokumentację.
Commit: `77ddc0c9046faece6781a294dd59df7febe02feb`.
GitHub Actions **Rust CI** `38373890158` i **Supply Chain and
Provenance** `38373889928` zakończyły się sukcesem, włącznie
z publikacją atestacji pochodzenia binarnego i SBOM. Późniejszy
priorytet 0.32 wprowadził odrębną weryfikację tych atestacji przez
konsumenta. Szczegóły: [docs/SUPPLY_CHAIN.md](docs/SUPPLY_CHAIN.md).
To nie jest certyfikacja produkcyjna.

## Prawa autorskie i kontakt

**© 2026 MOJEALTEREGO — Andrzej Mikulski. Wszystkie prawa zastrzeżone.**

**Email:** mojealterego21@gmail.com  
**Tel.:** +48 455 575 337

Nota o prawach autorskich nie zastępuje aktualnej deklaracji licencji MIT
w `Cargo.toml`. Warunki dalszej dystrybucji wymagają spójnego ustalenia
przed wydaniem produkcyjnym.


## Priorytet 0.32 — weryfikacja wydania i atestacji

GitHub Actions uruchamia dodatkowy etap `release-evidence` po pomyślnym
zbudowaniu artefaktu na `main`: pobiera konkretny artefakt, weryfikuje
SHA-256, strukturę archiwum i SBOM CycloneDX 1.5 oraz kryptograficznie
sprawdza podpisane pochodzenie kompilacji i atestację SBOM dla konkretnego
commita. Nie tworzy automatycznie tagów ani wydań. Dokumentacja:
[docs/RELEASE_EVIDENCE_032.md](docs/RELEASE_EVIDENCE_032.md).
**Stan wdrożenia produkcyjnego: NO-GO do czasu testów infrastrukturalnych.**


## Priorytet 0.33 — testy PostgreSQL na trzech hostach

Wprowadzono opt-in `ocaf-ha-probe` (`ha-probe`) oraz
`scripts/ha_multihost_acceptance.py`: skrypt operatora weryfikuje
w odizolowanej bazie na trzech hostach testowych wykluczanie liderów
i monotoniczne epoki fencing. Kontrole CI obejmują tylko osobne
procesy na jednym runnerze — nie są certyfikacją rzeczywistej
infrastruktury. Dokumentacja:
[docs/MULTIHOST_ACCEPTANCE_033.md](docs/MULTIHOST_ACCEPTANCE_033.md).
**Produkcja pozostaje NO-GO.**


## Priorytet 0.34 — kontrola cyklu życia certyfikatów mTLS

Dodano read-only walidator `scripts/mtls_lifecycle.py` do weryfikacji
łańcucha zaufania, EKU, identyfikatora SAN, dopasowania klucza prywatnego
oraz wymaganego okresu ważności certyfikatu. Dla kandydatów rotacji
sprawdzana jest też zmiana odcisku SHA-256 certyfikatu. Kontrole
operacyjne zabezpieczają pliki konfiguracji i certyfikatów przed
symlinkami w ostatnim komponencie oraz niebezpiecznymi uprawnieniami
plików kluczy. Testy rzeczywistych certyfikatów OpenSSL są wykonywane
przed podpisywaniem artefaktów w CI.
Dokumentacja: [docs/MTLS_LIFECYCLE_034.md](docs/MTLS_LIFECYCLE_034.md).

Ta funkcja **nie** rotuje certyfikatów automatycznie, nie sprawdza CRL/OCSP
ani nie wiąże SAN z tożsamością RPC. Wymagane są odrębne testy na hostach
operacyjnych i zatwierdzenie zmiany; produkcja nadal **NO-GO**.


## Priorytet 0.35 — kontrolowana wymiana kluczy HMAC agentów

Brama RPC obsługuje opcjonalną rotację klucza HMAC dla konkretnego
`sender`: stary klucz pozostaje ważny do granicy czasowej, nowy jest
ważny od zadanego momentu aktywacji, a dozwolone nakładanie kluczy
jest ograniczone do 30 minut. Odpowiedzi używają tego samego klucza,
który uwierzytelnił żądanie; po wycofaniu stary klucz jest
**odrzucany**. Polityka ACL, ochrona przed replay i limity dla agenta
pozostają obowiązujące niezależnie od rotacji.

Plik `OCAF_PEERS_FILE` przyjmuje w trybie jawnie włączonej rotacji
`next_key_b64`, `activate_at_ms` i `retire_old_at_ms`.
Niekompletne konfiguracje oraz powtórne użycie kluczy między
tożsamościami są odrzucane. Zmiana pliku wymaga restartu procesu.

**Ograniczenie:** sama rotacja HMAC nie dowodzi związania `sender`
z certyfikatem mTLS. Niezależna implementacja zweryfikowanego
powiązania TLS → RPC nadal jest wymagana. Szczegóły:
[docs/PEER_CREDENTIAL_ROTATION_035.md](docs/PEER_CREDENTIAL_ROTATION_035.md).
**Produkcja nadal NO-GO.**


## Architektura docelowa po audycie 26 plików PDF

Zintegrowana mapa zgodności dokumentacji z faktycznym kodem Rust,
zweryfikowane luki, ograniczenia obietnic oraz zamknięty **plan
priorytetów 0.36–0.50** znajdują się w
[docs/MASTER_ARCHITECTURE_26_PDFS.md](docs/MASTER_ARCHITECTURE_26_PDFS.md).
Przyjęto osobne cele odbioru dla bezpiecznego runtime (0.40)
i rozszerzonej platformy (0.50). Produkcja pozostaje **NO-GO**.


## Priorytet 0.36 — dodatkowy kryptograficzny dowód tożsamości agenta

Opcjonalna brama certyfikatu X.509 wymaga od przypisanego agenta
**zarówno poprawnego HMAC**, jak i podpisu żądania RPC kluczem
prywatnym odpowiadającym certyfikatowi o odcisku SHA-256
przypiętym w `OCAF_PEERS_FILE`. Weryfikacja podpisu następuje
**przed** przyjęciem numeru replay, kontrolą stawek i wywołaniem
narzędzia. Włączenie dla wszystkich agentów można wymusić przez
`OCAF_REQUIRE_PEER_CERT_POP=1` na koordynatorze. Po stronie klienta
`ocaf-rpc-node` używa `OCAF_RPC_CERT_KEY_FILE` ze
ściśle chronionym kluczem prywatnym.

**Ograniczenie:** to dowód posiadania klucza certyfikatu
na poziomie wiadomości, **a nie** niezależnie zweryfikowane
powiązanie z certyfikatem faktycznej sesji TLS Zenoh.
Tożsamość transportowa, łańcuch CA, EKU, SAN, revocation
i rzeczywiste TLS channel-binding wymagają odrębnej
weryfikacji i implementacji. Patrz
[docs/RPC_CERT_IDENTITY_036.md](docs/RPC_CERT_IDENTITY_036.md).

## Pełna Roadmapa: 26 plików PDF + 4 dodatkowe wersje badań

**[OCAF Full Ecosystem Roadmap — priorytety 0.36–0.94](docs/ROADMAP_FULL_ECOSYSTEM_2026.md)**
obejmuje 26 dokumentów wcześniejszych i 4 dodatkowe pliki PDF
(zawierające wersjonowane powtórzenia i rozszerzenia), rozdziały 1–33,
nowe eksperymenty 2027–2033 oraz moduły
wiedzy, pamięć, bezpieczeństwo, ekonomię, Agents Builder
(Cloud/Desktop/Mobile Code & No-Code), OmniMath oraz
**każdy wyspecjalizowany rój agentowy**. Etapy mają
mierzalne kryteria akceptacji. Dotychczasowa mapa 0.36–0.50
jest nieaktualna i zostaje zastąpiona tą roadmapą.
Stan wdrożenia produkcyjnego: **NO-GO**.


## Priorytet 0.37 — TLS 1.3: powiązanie rzeczywistego certyfikatu sesji z RPC

Dodano opcjonalną bramę `tls-bound`, opartą o TCP + rustls/Tokio,
która odczytuje certyfikat klienta **z zweryfikowanej sesji mTLS
przenoszącej dane RPC**, a nie z informacji deklarowanych przez agenta.
Następnie wymaga zgodności SHA-256 certyfikatu z przypisanym
`sender`, poprawnego DNS SAN, HMAC i podpisu PoP z 0.36.

`OCAF_RPC_INGRESS=tls-bound` uruchamia alternatywny listener i
**nie otwiera żadnego queryable Zenoh** w danym procesie koordynatora.
Funkcja jest opt-in (`--features tls-bound`); standardowe wdrożenia
Zenoh nie uzyskują automatycznie tego mechanizmu. Wstępny klient CLI
obsługuje ten transport; demo worker/server go nie obsługuje i
zamiast fallbacku odmawia pracy. Brakuje automatycznej kontroli
CRL/OCSP i odbioru na niezależnych hostach.

Procedura: [docs/TLS_PEER_IDENTITY_037.md](docs/TLS_PEER_IDENTITY_037.md).
**Produkcja: NO-GO**. Plan wszystkich rojów bez zmian:
[Roadmapa 0.36–0.94](docs/ROADMAP_FULL_ECOSYSTEM_2026.md).

## Aktualizacja roadmapy przed etapem 0.38 (2026-10-13)

**Stan historyczny sprzed rozpoczęcia implementacji 0.38:**
mapa drogowa została zaktualizowana przed wdrożeniem sandboxa. Priorytety **0.36–0.94** obejmują rdzeń,
wszystkie wyspecjalizowane roje (0.56–0.71), platformy budowania
agentów, weryfikację oraz nowe badania OmniProof-STARK,
OmniMacro, OmniMarket-L402+, OmniSubstrate, proof-carrying RSI,
OmniThermo-Governor, neuromorfikę, wetware, optykę i kryptografię
topologiczną — z oddzieleniem hipotez od sprawdzonych wdrożeń.

**Aktywny etap: 0.38 — WASI sandbox default-deny**.
Szczegółowa kolejność techniczna, zależności, modele zagrożeń
oraz negatywne testy:
[docs/PRIORITY_038_EXECUTION_CONTRACT.md](docs/PRIORITY_038_EXECUTION_CONTRACT.md).
Nie mylić `ToolExecutor` z izolacją obcego kodu ani
`tokio::timeout` z limitem CPU/RAM. Żaden obcy kod nie dostanie
praw hosta, plików ani sieci wyłącznie na podstawie deklaracji agenta.

**Plan aktualny:** [docs/ROADMAP_FULL_ECOSYSTEM_2026.md](docs/ROADMAP_FULL_ECOSYSTEM_2026.md).
**Status wdrożeniowy:** Stage 0.38 PARTIAL / SECURITY REVIEW REQUIRED; Production **NO-GO**.

## Priorytet 0.38 — WASI Preview1: pierwsza implementacja (Linux)

Dodano `src/executor/wasi.rs`, który realizuje osobne,
ograniczone wykonanie modułu WebAssembly poprzez przypięty
SHA-256 plik binarny `wasmtime run`. Adapter
`WasmToolExecutor` implementuje istniejący `ToolExecutor`.
Wymaga jawnego operatorowego przekazania zatwierdzonego
SHA-256 modułu i silnika, nie wykonuje nieznanych importów,
nie daje WASI dostępu do plików hosta ani sieci oraz
egzekwuje limity paliwa, pamięci liniowej, stosu,
wall-clock, rozmiaru JSON i liczby procesów.

Nowe testy `tests/wasi_sandbox.rs` i krok Rust CI pobierający
archiwum Wasmtime 49.0.2 z przypiętym upstream SHA-256
obejmują prawdziwy program WASIp1 i negatywne przypadki.
Szczegóły, procedura i ograniczenia:
[docs/WASI_EXECUTION_038.md](docs/WASI_EXECUTION_038.md).

**0.38 pozostaje PARTIAL / NO-GO.** Implementacja obejmuje teraz
operatorowo podpisane manifesty Ed25519, przypisanie `tenant:agent`,
worker TLS-bound, podpisy HMAC `ExecutionReceipt` oraz sprawdzanie
plików konfiguracyjnych przy użyciu deskryptorów z `O_NOFOLLOW`
i kontrolą uprawnień/inode (z testami negatywnymi). Nadal wymagane
są niezależne testy odporności na awarie, obciążenie i wycieki,
audyt hosta, rotacja kluczy/PKI oraz formalny odbiór bezpieczeństwa.
Etap 0.39 został rozpoczęty na wyraźne polecenie operatora, wyłącznie w zakresie fail-closed admission Firecracker/KVM + Aya/eBPF i odczytowego host preflight. Szczegóły: [docs/STAGE039_MICROVM_EBPF.md](docs/STAGE039_MICROVM_EBPF.md). Ani 0.38, ani 0.39 nie mają produkcyjnego GO.

## Odbiór bezpieczeństwa 0.38.8–0.38.9

Dodano cleanup całej linuxowej grupy procesów WASI, blokadę
eskalacji `no_new_privs`, `RLIMIT_CORE=0`, negatywne scenariusze
stderr/descendant/zmienione moduły oraz rozszerzony test
rzeczywistego Wasmtime. Zestaw kryteriów, reprodukcji i
otwartych ryzyk: [docs/STAGE_038_8_9_ACCEPTANCE.md](docs/STAGE_038_8_9_ACCEPTANCE.md).
**Etap 0.38 pozostaje PARTIAL / NO-GO** do niezależnego
odbioru i wymaganego hostowego hardeningu.

### 0.38 — host acceptance, sealed engine i dostęp operatora

Signed WASIp1 wykonuje przypiętą kopię Wasmtime przez uszczelniony Linux `memfd` zamiast wykonywać mutowalną ścieżkę pliku; system Linux uzupełniają `PR_SET_PDEATHSIG`, grupy procesów i `no_new_privs`. `scripts/wasi_host_acceptance.py` dokumentuje wyniki testów obciążenia i obserwowane RSS/FD/PID w JSON. Konfiguracja hostowa systemd/cgroup to [szablon, nie wdrożenie](deploy/systemd/ocaf-coordinator-wasi.service.example). Kody, ograniczenia i braki dostępu: [dokumentacja odbioru](docs/STAGE_038_8_9_ACCEPTANCE.md). **0.38 PARTIAL / NO-GO, 0.39 zablokowany.** Plan zewnętrznego odbioru: [IBM Cloud VPC](docs/IBM_CLOUD_STAGE_038.md) oraz [warunkowy 30-dniowy pilotaż OVHcloud Public Cloud](docs/OVHCLOUD_30_DAY_STAGE_038_PLAN.md). Dodatkowe testy po weryfikacji ofert: [audyt 8 darmowych/trial hostów](docs/CLOUD_FREE_TRIALS_OCAF_2026.md) i [Oracle Cloud: lokalna automatyzacja read-only i bramki płatności](docs/ORACLE_CLOUD_AUTOMATION.md). Promocyjne kredyty nie stanowią potwierdzenia wdrożenia ani budżetowego limitu bezpieczeństwa.


## Etap 0.39 — mikroVM Firecracker/KVM i Aya/eBPF (rozpoczęty)

**Zaimplementowano:** `scripts/stage039_microvm_gate.py` — fail-closed weryfikacja manifestu, podpisanych digestami SHA-256 artefaktów, deny-by-default polityki (jailer, brak sieci, RO rootfs, seccomp) i read-only host prerequisites (`/dev/kvm`, cgroup v2, systemd, bpffs, BTF). Testy: `scripts/tests/test_stage039_microvm_gate.py`, CI: `.github/workflows/stage039-microvm-preflight.yml`. **Nie uruchamia VM ani Aya/eBPF**; artefakty CI nie dowodzą izolacji. `0.38 ADMINISTRATIVELY CLOSED (operator directive), odbiór techniczny A01–A12 NO-GO`; `0.39 IN PROGRESS / NO-GO`; produkcja **NO-GO**.


### 0.39.2 — lokalny ledger brokera (transakcje i fencing)

`scripts/stage039_broker_ledger.py` dodaje trwałe rezerwacje SQLite WAL,
anty-replay `request_id`, wymaganie monotonicznego `leader_epoch`,
własność `tenant:agent:workload`, TTL i bounded-capacity przy sprawdzonym
pinowaniu manifestu i gotowości hosta. Nową bazę tworzy z mode 0600
jeszcze przed wywołaniem `sqlite3.connect`. Testy: 
`scripts/tests/test_stage039_broker_ledger.py`, 17 regresji na faktycznym SQLite,
obok 12 testów statycznego admission. **To bramka admission, a nie
rzeczywisty broker Firecracker ani uruchomiony Aya/eBPF.** 
Dyspozycja operatora o administracyjnym zamknięciu 0.38:
[docs/STAGE038_OPERATOR_DIRECTIVE_2026-10-15.md](docs/STAGE038_OPERATOR_DIRECTIVE_2026-10-15.md).
Weryfikacja produkcyjna pozostaje zależna od rzeczywistych testów
i raportu bezpieczeństwa; release gate nie jest omijany automatycznie.


### Aktualizacja 2026-10-15 — odblokowanie decyzji operatora i 0.39.3

Na wyraźne polecenie operatora **blokada administracyjna dopuszczenia do
próby wydania została zdjęta** w
[`ops/OCAF_RELEASE_OPERATOR_DECISION_2026-10-15.json`](ops/OCAF_RELEASE_OPERATOR_DECISION_2026-10-15.json).
W `scripts/release_readiness.py` dostępna jest świadoma, jawna
ścieżka `--operator-override-policy` oraz
`--ack-unverified-production-risk`. Pozostałe wyniki A01–A12,
status weryfikacji 0.38 oraz brak rzeczywistego wdrożenia pozostają
udokumentowane bez nadpisywania ich fałszywym PASS. Samo zdjęcie blokady
operatora **nie wdraża oprogramowania** i nie poświadcza środowiska.

W 0.39.3 dodano `scripts/stage039_broker_ipc.py`, realny
`AF_UNIX` z kernelowym `SO_PEERCRED`, ograniczonymi wiadomościami,
polityką jednego tenant:agent na socket oraz operacjami admission/release
na lokalnym SQLite. Zestaw regresji 0.39 obejmuje 12 testów admission,
17 testów trwałych rezerwacji oraz 10 testów realnego IPC.
Rzeczywistej VM Firecracker i attach Aya/eBPF jeszcze nie wykonano.


### Aktualizacja roadmapy v5 / Firecracker + Aya (15.10.2026)

Z dwóch dołączonych opracowań o **24 i 9 repozytoriach** powstała
[macierz 33 adaptacji](docs/ROADMAP_33_REPOSITORY_ADAPTATIONS_2026-10-15.md)
z odwołaniami do istniejących etapów i testami negatywnymi.
[Pełna Roadmapa v5](docs/ROADMAP_FULL_ECOSYSTEM_2026.md) rozpisuje
`0.39.1–0.39.11`.

W `0.39.4–0.39.7` dodano: rzeczywisty probe `KVM_GET_API_VERSION`
i `KVM_CREATE_VM` (`scripts/stage039_kvm_ioctl.py`),
fail-closed generator zweryfikowanej konfiguracji jailer/Firecracker
(`scripts/stage039_firecracker_spec.py`) oraz źródła
`aya-ebpf` deny-all egress i skompilowany userspace Aya loader
(`stage039/aya-guard`). CI uruchomiło **68 testów PASS**
([workflow](https://github.com/mojealterego/ocaf-runtime/actions/runs/38663469207)).
Aktualny host **nie ma działającego KVM**; Firecracker nie został
uruchomiony, Aya nie został podpięty, brak produkcyjnej izolacji.


### 0.39 — testy rzeczywistego Firecracker i Aya (inkrementacja 2026-10-15)

Zaimplementowano `scripts/stage039_live_firecracker.py` uruchamiający
**prawdziwy jailer/Firecracker** wyłącznie po udanym kernel `KVM_CREATE_VM`,
z weryfikacją API `Running`, konsoli gościa i członkostwa cgroup; oraz
`scripts/stage039_live_egress.py` wykonujący test A/B na lokalnym
TCP `127.0.0.1`: A ma utracić egress po rzeczywistym Aya attach,
B zachować łączność, A odzyskać ją po detach. Workflow
`.github/workflows/stage039-live-kvm-aya.yml` wymaga własnego,
uprawnionego self-hosted runnera KVM/BPF. **84 testy kontraktu PASS** w
[GitHub Actions](https://github.com/mojealterego/ocaf-runtime/actions/runs/38665774854),
ale testy sprzętowe nie są wykonane. Dostępne kontenery/Sprite
nie udostępniają `/dev/kvm`, aktualny status izolacji: **NO-GO**.


### Aktualizacja 0.39.7 — Aya eBPF ELF zweryfikowany i gotowy do użycia na hoście

`stage039/aya-ebpf-egress` kompiluje się do natywnego
`bpfel-unknown-none` (test `readelf` PASS, SHA-256
`992cd7a365067f52bce5afe5ca9758dad0e15ad9291b71991914040faaa55c95`,
992 B). Pobrano i zweryfikowano `bpf-linker v0.11.0` po
SHA-256; odrębny eBPF `Cargo.lock` jest w `main` i kolejny
CI buduje z `--locked`. Wynik
[GitHub Actions 38666442623](https://github.com/mojealterego/ocaf-runtime/actions/runs/38666442623)
= **SUCCESS na cross-compile**, nie sukces sprzętowego
`BPF_PROG_LOAD`/attach, negatywnego testu ruchu lub boot Firecracker.
Self-hosted workflow `.github/workflows/stage039-live-kvm-aya.yml`
pozostaje ścieżką rzeczywistych testów KVM/BPF.


### 0.39 — sprzętowe wyniki testów GitHub-hosted Linux (15.10.2026)

**Rzeczywisty KVM API=12 i KVM_CREATE_VM: PASS.**
**Rzeczywisty Aya cgroup egress deny/recovery A/B: PASS.**
Zadziałało dopiero `sudo -n` na hostowanym Ubuntu 24.04.
[Dowód źródłowy](docs/STAGE039_REAL_KVM_AYA_PROOF_2026-10-15.md),
[CI 38704086975](https://github.com/mojealterego/ocaf-runtime/actions/runs/38704086975).
Jailer Firecracker i guest boot są testowane oddzielnie i nie
są jeszcze poświadczone. GH runner ephemeral nie jest produkcją.


### 0.39 — potwierdzony Linux guest boot w jailer Firecracker (2026-10-15)

Na `main`, commit [`677bd3e7eea0`](https://github.com/mojealterego/ocaf-runtime/commit/677bd3e7eea055af9fe6a4b6259c706b5eddcb17),
poprawiono ramkowanie HTTP `Content-Length` w `_api_state` i
uzyskano **PASS rzeczywistego kernel boot przez Firecracker 1.17.0
w jailer na KVM** na uprzywilejowanym, efemerycznym runnerze
([GitHub Actions 38710048957](https://github.com/mojealterego/ocaf-runtime/actions/runs/38710048957)).
Szczegółowy [raport dowodowy](docs/STAGE039_REAL_FIRECRACKER_BOOT_PROOF_2026-10-15.md)
oddziela boot/cgroup od rzeczywistego Aya attach i końcowego
testu izolacji agentów. **0.39 W TOKU / produkcja NO-GO**; 0.38
administracyjnie zamknięty, techniczny odbiór niekompletny.


### 0.39 — jednoczesna próba Firecracker + Aya w cgroup uruchomionej VM (2026-10-15)

**PASS (GitHub-hosted Ubuntu 24.04):** w trakcie rzeczywistego działania
gościa Linux w jailed Firecracker/KVM podpięto Aya `cgroup_skb/egress`
do **dokładnie tej samej hostowej grupy cgroup, co proces VM**.
Hostowy próbnik A w tym cgroup traci TCP do `127.0.0.1` po attach,
niezależny cgroup B zachowuje połączenie, A odzyskuje po detach;
VM działa w czasie próby. [Wynik CI 38711385989](https://github.com/mojealterego/ocaf-runtime/actions/runs/38711385989),
[raport i integralność](docs/STAGE039_COMBINED_VM_CGROUP_EGRESS_PROOF_2026-10-15.md).

**Granica dowodu:** połączenia inicjuje proces **hostowy** dołączony
do cgroup Firecracker, nie kod gościa. Obraz VM jest bez sieci,
dlatego nie testowano pakietów emitowanych z maszyny wirtualnej.
Nie jest to produkcyjny GO ani audyt dwóch niezależnych hostów.
Etap 0.38 ma status roadmapy **COMPLETE (decyzja operatora)**,
odbiór bezpieczeństwa A01–A12 pozostaje **INCOMPLETE**.


### 0.39.8–0.39.9 — egzekwowane limity host-cgroup i próba SIGKILL (2026-10-15)

**Rzeczywiste, pozytywne testy na GitHub-hosted Linux/KVM:**
[Actions 38713156606](https://github.com/mojealterego/ocaf-runtime/actions/runs/38713156606).
Nowy `scripts/stage039_vm_resources.py` stosuje dla każdej efemerycznej
VM przed startem jailera wymuszone w jądrze cgroup v2 limity:
`memory.max` (dla 256 MiB RAM gościa: 768 MiB hosta), `memory.swap.max=0`,
`memory.oom.group=1`, `pids.max=96`, `cpu.max=200000 100000`.
Dodatkowo weryfikuje realny odczyt limitów i `memory.peak`,
`pids.peak`, `memory.events`, `pids.events`, `cpu.stat`.
Wystąpienie OOM lub uderzenie w limit PID oznacza odmowę odbioru.

`scripts/stage039_vm_crash.py` wykonuje **rzeczywisty SIGKILL** wobec
własnego sprawdzonego procesu jailed Firecracker, weryfikuje kod zakończenia
`-SIGKILL` i jądrowe `cgroup.events: populated=0` (również potomne
grupy) przed usunięciem cgroup. Istnieją osobne testy odmowy, regexów i
fałszywych zdarzeń. [Raport z wynikami i ograniczeniami](docs/STAGE039_VM_RESOURCE_FAULT_PROOF_2026-10-15.md).
**0.38 = COMPLETE (decyzja operatora), 0.39 = IN PROGRESS, produkcja = NO-GO.**
Próba nie dowodzi odporności na awarię całego hosta ani izolacji
ruchu emitowanego z wnętrza VM (gość bez interfejsu sieciowego).


### 0.39 — rzeczywiste testy dwóch tenantów, ruchu guest UDP i oddzielnych hostów CI (2026-10-15)

Zrealizowano **PASS dwóch równoczesnych jailed VM Firecracker/KVM**
o różnych tenantach i inode cgroup; VM B pozostaje sprawna po SIGKILL VM A.
W odrębnym rzeczywistym teście **dwa goście Firecracker inicjują UDP**
na oddzielnych TAP; `nftables` blokuje tylko A, B działa, po zdjęciu
blokady A odzyskuje ruch. [CI 38715926542](https://github.com/mojealterego/ocaf-runtime/actions/runs/38715926542).

W dwóch oddzielnych jobach GitHub-hosted Linux uzyskano różne kernel
`boot_id`, niezależne `KVM_CREATE_VM`, boot jailed Firecracker,
SIGKILL VM i opróżnienie cgroup na **obu hostach**; osobny weryfikator
porównał dowody i zakończył się PASS:
[CI 38720667372](https://github.com/mojealterego/ocaf-runtime/actions/runs/38720667372).
Szczegółowy [raport](docs/STAGE039_TWO_TENANTS_GUEST_NETWORK_INDEPENDENT_HOSTS_2026-10-15.md).

**Rozgraniczenie:** test UDP korzystał z gości **bez jailer** i blokady
`nftables`, nie Aya guest egress. Dwa hosty to efemeryczne VM CI,
nie produkcyjna infrastruktura HA. Nie dowiedziono rzeczywistej partycji
między hostami, przełączenia usługi, host power loss lub podpisanego
niezależnego audytu. **0.38 COMPLETE (decyzja operatora);
0.39 IN PROGRESS; produkcja NO-GO.**


### 0.39.12–0.39.14 — real jailed guest UDP + kernel TC eBPF + cross-kernel checkpoint (2026-10-15)

**CI PASS**: two simultaneous isolated jailed Firecracker tenant VMs originate UDP from their own guest PID1 via distinct TAPs; nftables deny/recovery, tenant B survives tenant A SIGKILL, and a kernel **TC ingress eBPF** program attached specifically to tenant A TAP blocks **actual guest-originated UDP** while B remains reachable; after detach A recovers. [Real KVM TC/eBPF proof 38722682521](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722682521).

**CI PASS**: two separate hosted Linux kernels, first kills jailed guests, then a second boots new jailed guests and emits the same source-checkpoint nonce after a bounded, SHA-256-correlated GitHub artifact handoff. [Cross-kernel cold restart 38722139701](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722139701).

These are **ephemeral CI** tests; TC BPF is kernel classifier loaded by `tc` (not Aya userspace attach). Artifact-mediated cold restart is not real-time two-host mTLS RPC/failover. Independent audit and signed production/operator acceptance remain absent. **0.39 IN PROGRESS / production NO-GO.** [Detailed proof ledger](docs/STAGE039_NETWORK_EBPF_CROSS_HOST_EVIDENCE_2026-10-15.md).

### Stage 0.39 — real TLSv1.3 mTLS fencing and signed dossier verification (2026-10-15)

New experimental work validated a **real mutual-certificate TLS1.3 TCP RPC** over separate Linux network namespaces connected by a kernel veth pair. The lab injected a Linux blackhole route network partition, denied rogue CA-signed client identity and competing leader lease, and verified SIGKILL/persistent SQLite fencing recovery. [Real privileged Linux workflow PASS 38724640542](https://github.com/mojealterego/ocaf-runtime/actions/runs/38724640542). This is **one kernel** and not a production two-host HA test. An additional OpenSSL/Ed25519 verifier validates out-of-band pinned independent signer keys and signed evidence hashes; CI uses explicitly **synthetic** keys and cannot manufacture external audit or operator signatures. [Consolidated factual 0.39 evidence and blockers](docs/STAGE039_ACTUAL_BOUNDARY_CLOSURE_2026-10-15.md). **0.39 IN PROGRESS; release NO-GO.**

### Continuous roadmap implementation mandate — 0.40 and 0.41 actual PostgreSQL labs (2026-10-15)

Operator authorized unrestricted **development progression and roadmap extension without separate external audit/operator approval steps**; implementation claims remain evidence-backed and production status is kept separate. [Mandate](ops/OCAF_CONTINUOUS_DEVELOPMENT_POLICY_2026-10-15.json). **Stage 0.40 REAL local PostgreSQL three-postmaster WAL replication + crash/promotion/reparent:** [CI 38726639320 PASS](https://github.com/mojealterego/ocaf-runtime/actions/runs/38726639320). **Stage 0.41 REAL PostgreSQL custom dump SHA-256, tamper denial, exact restore:** [CI 38727035810 PASS](https://github.com/mojealterego/ocaf-runtime/actions/runs/38727035810). Both are one-kernel labs; do not infer 3 physical hosts or inter-host DR. [Detailed evidence ledger](docs/STAGE040_041_REAL_POSTGRES_LABS_2026-10-15.md).

### 0.41.2: two real independent Linux-kernel PostgreSQL snapshot recovery (2026-10-15)

**CI PASS:** two separate Ubuntu runner kernels, source PostgreSQL 16 `pg_dump --format=custom` after 25 durable receipts and stopped local server, destination SHA-256 verified `PGDMP`, **actual fresh `initdb` + `pg_restore`**, exact 25-row content SHA256 match. On the destination host, intentionally corrupted backup bytes and a reused source kernel `boot_id` were each rejected. [Actual run 38734668015, two jobs SUCCESS](https://github.com/mojealterego/ocaf-runtime/actions/runs/38734668015), [full evidence and limits](docs/STAGE041_CROSS_KERNEL_RESTORE_2026-10-15.md). Ephemeral runner recovery ≠ production DR or zero continuous RPO; release remains NO-GO.

### 0.41.3: REAL PostgreSQL archived-WAL point-in-time recovery (2026-10-15)

**HARDWARE CI PASS** [38737008767](https://github.com/mojealterego/ocaf-runtime/actions/runs/38737008767): write 25 records, run physical `pg_basebackup`, commit 10 more and capture genuine WAL LSN, then commit another 5 unwanted records, force WAL archival, IMMEDIATE-stop PostgreSQL primary and start a fresh postmaster with `recovery.signal` / `restore_command` / `recovery_target_lsn`. The recovered database contains **35/35 intended records and zero records committed after target LSN**, with exact SHA-256 content equality. One ephemeral Linux host, not actual physical host loss or off-site archive. [Full evidence ledger](docs/STAGE041_REAL_WAL_PITR_2026-10-15.md).

### 0.41.4: real PostgreSQL physical WAL PITR on two distinct Linux kernels (2026-10-15)

**HARDWARE CI PASS**: [two independent GitHub Actions Linux jobs 38738198412](https://github.com/mojealterego/ocaf-runtime/actions/runs/38738198412). Source PG16 crashes via `pg_ctl stop -m immediate` after archiving WAL and 5 post-target commits; destination validates two SHA256-pinned physical basebackup/WAL TARs, rejects a deliberately tampered WAL archive and replayed `boot_id`, boots fresh PostgreSQL from actual basebackup and WAL replay to target LSN with **35/35 intended rows, 0 post-target records and identical SHA256**. Not physical power loss, encrypted offsite storage, enterprise DR or production GO. [Full evidence](docs/STAGE041_TWO_KERNEL_PHYSICAL_WAL_PITR_2026-10-15.md).

### 0.39.18: ACTUAL SO_PEERCRED broker admission before two JAILED Firecracker VM boots (2026-10-15)

**Real kernel CI PASS:** [38739812121](https://github.com/mojealterego/ocaf-runtime/actions/runs/38739812121), [admission regression 38739812535](https://github.com/mojealterego/ocaf-runtime/actions/runs/38739812535). Two separate AF_UNIX broker sockets authenticate real SO_PEERCRED UID/tenant principals and persist distinct manifest SHA256-bound workload reservations **before** booting both actual jailed Firecracker Linux guests; VM ID, boot, cgroup inode, guest/rootfs SHA, Aya host-cgroup and SIGKILL evidence are bound to the reserved identities, and both leases released. **Signed WASI execution inside a Firecracker guest still NOT PROVEN.** [Proof ledger](docs/STAGE039_040_BROKER_KERNEL_PG_INDEPENDENCE_2026-10-15.md).

### 0.40.1: three independent Linux runner kernel PostgreSQL crash/failover labs (2026-10-15)

**Four-job hardware CI PASS:** [38740089797](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740089797). Three separate Linux kernel boot IDs each run real PostgreSQL A/B/C local streaming, primary crash and standby reparent; independent fourth job rejects coincident kernel IDs and false release claims. **This does not establish live replication between the three different hosts.** [Evidence ledger](docs/STAGE039_040_BROKER_KERNEL_PG_INDEPENDENCE_2026-10-15.md).

### 0.40.2: actual PostgreSQL WAL streaming and crash after 30s/8-client pgbench reliability stress

**Real CI PASS:** [38740936947](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740936947). PostgreSQL 16 primary A with streaming B/C, a real `pgbench -i` dataset, **8 concurrent clients / 2 threads / 30 seconds** WAL churn; both replicas confirmed matching accumulated account/branch/teller balances, then A IMMEDIATE process crash, B promotion and C reparent with resumed streaming. PostgreSQL fast checkpoint recovery of `pg_basebackup` was corrected after an initial stressed timeout. [Evidence and limits](docs/STAGE039_040_BROKER_KERNEL_PG_INDEPENDENCE_2026-10-15.md). All 3 local DB processes share one Linux kernel, so not a durability claim across physical servers.

### 0.40.6: genuine PostgreSQL WAL over private kernel TCP/veth namespaces with live partition (2026-10-15)

**REAL CI PASS** [38752910618](https://github.com/mojealterego/ocaf-runtime/actions/runs/38752910618): three actual PostgreSQL 16 postmasters using **separate Linux network namespaces and veth/bridge**, each with its own private IP; source A's `pg_stat_replication` sees `10.247.40.2` and `10.247.40.3`. Kernel route blackholes B→A, C continues replay, B freezes, route heals and B catches up. After A IMMEDIATE crash, B promotes and C re-parents by **live TCP**. The kernel is SHARED, so this is not PostgreSQL replication between independent physical hosts/kernels. [Evidence](docs/STAGE040_LIVE_TCP_NETNS_PARTITION_2026-10-15.md).


### Roadmap v8 — source-traceable Apex Tier-0 extension (2026-10-15)

Added **40 further main stages (1.15–1.54) and 12 infrastructure subtasks** directly to `main`, all backed by positive/negative acceptance tests, dependencies, responsible subsystems and page references to the newly supplied 15-page `Dokument bez nazwy(2).PDF`. Six source pillars: Zenoh SHM and hybrid Automerge/Yrs; CoALA/UALA/Titans; RQGM/type-sitter; Sigma-Delta/Loihi 2 and ML-KEM; genuine guest Aya eBPF and FinOps; L402/Macaroon/Lightning. **Status of ALL new features: PLANNED, not implemented.** Numerical performance claims from the supplied PDF require actual measurements and cannot be treated as verified. [Full v8 table](docs/ROADMAP_APEX_TIER0_2026-10-15.md) · [Machine-readable 52-stage scope](docs/roadmap/apex_tier0_extension.json). Python regression and CI block missing rows, invented test evidence and false production approval.

### OCAF Roadmap v9 (2026-10-15)

Added **40 main roadmap stages 1.55–1.94 plus 16 technical subtasks**, reflecting the eight-step research/deployment program and six-Crate Cargo Workspace proposal in the user's newer 13-page research PDF. The total main stage identifier range is **195 (0.00–1.94)**; added functions remain **PLANNED**, and source benchmark numbers are not OCAF empirical results. [Detailed 56 rows](docs/ROADMAP_SOVEREIGN_WORKSPACE_V9_2026-10-15.md) · [machine manifest](docs/roadmap/sovereign_workspace_v9.json). Existing KVM WASI-in-guest and independent-host PostgreSQL HA requirements stay OPEN until actual tests succeed.

### Iteracja wykonawcza 0.39–0.41 — rzeczywiste testy i granice infrastruktury (2026-10-15)

**0.39: Guest WASIp1 / Firecracker remains IN PROGRESS.** In actual Firecracker KVM Linux 4.14 guests, Wasmtime stalled before `fd_write` due to an uninitialized guest kernel CRNG (console `OCAF039_GUEST_CRNG_NOT_INITIALIZED`, [failing run 38764740853](https://github.com/mojealterego/ocaf-runtime/actions/runs/38764740853)). Kernel-authenticated broker now waits for actual guest WASIp1 success before VM termination; a kernel virtio-rng entropy device was configured; additional dedicated old-kernel CPU RDSEED/RDRAND → Linux `RNDADDENTROPY` bootstrap and strict `getrandom` readiness checks are being exercised in real KVM. **No PASS is claimed until a new real runner finishes green.**

**0.40/0.41: REAL 20-second PostgreSQL network chaos PASSED.** [GitHub Actions 38764083703 — SUCCESS](https://github.com/mojealterego/ocaf-runtime/actions/runs/38764083703) ran three genuine PG postmasters in separate Linux network namespaces via veth/TCP, `pgbench -c8 -j2 -T20`, bidirectional A↔B blackhole, healthy C WAL replay, delayed B WAL rejoin and full account balance checksums, A IMMEDIATE crash, B promotion and C reparent. **All 3 processes share one kernel; not inter-host HA.**

**0.40.3: TRUE live 3-host PostgreSQL test infrastructure IMPLEMENTED but NOT EXECUTED.** [GitHub Actions 38765271549 — unit PASS, remote job SKIPPED](https://github.com/mojealterego/ocaf-runtime/actions/runs/38765271549). `scripts/stage040_interhost_live.py` requires three genuinely different Linux kernel boot IDs, SSH host-key pinning, real primary `pg_stat_replication.client_addr`, two real `pg_stat_wal_receiver` streams and a newly written source nonce/LSN replayed via WAL on both remote hosts. [Required topology and runbook](docs/STAGE040_REAL_INTERHOST_LIVE_PG_ACCEPTANCE_2026-10-15.md). An independently provisioned authorised three-host lab and operator SSH keys are not currently connected; therefore **true inter-host streaming and quorum fencing remain unverified**.

### PDF 09 — wymagania systemów agentowych i symulacji społecznych

Źródło: `Analiza AGI I Ludzkich AI (1).PDF`, s. 5–11. Kanoniczne szczegóły i kryteria odbioru: [ROADMAP.md](ROADMAP.md) oraz `docs/roadmap/canonical_v11_inventory.json`. Zakres: 2.03 Generative Agents/Lyfe/MetaMind/ToM-SWE (opt-in social simulations, intent hypotheses, sensitive data privacy); 1.98 DSPy (holdout optimization); 2.05 Agent-Zero/Hermes (supervised self-programming); 2.01 AI Scientist (reproducible experiments); 2.00 AutoGPT/BabyAGI (bounded task DAG); 2.06 GodMode (operator GUI/HITL); 2.02 OpenHands (sandboxed engineering); 1.99 SWE-bench Verified/mini-SWE-agent (N-run, regression and reproducibility). Wszystkie wymienione elementy są **wymaganiami planowanymi, nie potwierdzonymi integracjami**. Bez zmiany historycznej numeracji.


### Nowe etapy po 207 historycznych pozycjach

Dla wymagań PDF 09 utworzono **208–215**: 208 symulacje społeczne/ToM, 209 DSPy, 210 Agent-Zero/Hermes, 211 AI Scientist, 212 AutoGPT/BabyAGI, 213 GodMode, 214 OpenHands, 215 SWE-bench/mini-SWE-agent. Poprzednie etapy 1.98–2.06 pozostają historycznymi pozycjami badawczymi, nie są automatycznie ukończone. Następny numer: **216**. Status nowych etapów: PLANNED.


**PDF16 (2026-10-16):** 17 stron przypisano do istniejących etapów 0.39, 0.42–0.49, 0.55, 0.63–0.64, 0.67, 0.91 i 0.98. [Raport audytu](docs/audits/PDF16_OCAF_ARCHITECTURE_SEMANTIC_REVIEW_2026-10-16.md). Kryteria: OTLP trace continuity, MCP schema/injection, ZKP, Firecracker/eBPF, real L402 settlement, PQC handshake, RAG E2E, Loihi HIL. PLANNED/UNVERIFIED.


**PDF17 (18 stron, 2026-10-16):** [Audyt](docs/audits/PDF17_OCAF_RUST_ARCHITECTURE_SEMANTIC_REVIEW_2026-10-16.md). Dodatkowe luki: ACP/OpenHands adapter (0.43/0.44/0.55), Aider-style Tree-sitter repo-map (0.43/0.67), reproducible SWE-bench Verified coding-agent eval (0.51/0.98). PLANNED/UNVERIFIED.


**PDF18 (19 stron):** [audyt OmniDev](docs/audits/PDF18_OCAF_OMNIDEV_SEMANTIC_REVIEW_2026-10-16.md). Wymagania: Architect Tree-sitter, Security Critic, Firecracker Executor, Zenoh/CRDT, test-driven generation, recovery, E2E i benchmarki. Etapy 0.43, 0.46–0.47, 0.55, 0.63, 0.67, 0.98. PLANNED/UNVERIFIED.


## PDF19 — OmniUX i powiązanie z ekosystemem OCAF (21 stron; 2026-10-16)

Źródło: Analiza Architektury Systemu OCAF(5).PDF. Strony 1–16: Zenoh SHM, CRDT, OTLP, MCP/Tree-sitter, OAuth/ZKP, Firecracker/Aya, L402, RAG/ASI, PQC/Loihi, OmniDev — istniejące etapy 0.39, 0.42–0.49, 0.55, 0.63–0.64, 0.67, 0.91, 0.98. Strony 16–18: **OmniUX**, natywny rój UI/UX: DesignKognitywista_MCP (Figma MCP, OAuth, Auto Layout, design tokens), FrontendCSTGenerator (React/Svelte/Yew, typed design IR, AST/CST), WizualnyKrytykWasm (rendering DOM, testy responsywności, kontrastu/a11y, pixel-diff, izolacja Wasm/WebGPU/Firecracker). Etapy: 0.43–0.44, 0.50–0.55, 0.63, 0.67, 0.98. Wymagane E2E, bezpieczeństwo, trwałość projektów, round-trip Code/NoCode i realne testy na pięciu platformach. OmniUX to rój, a nie jedenasty Agents Builder. Przykładowy kod zwraca stałe FIGMA_TOKENS_MCP_AST, komponent Button i 0.99; nie jest wdrożeniem. Strony 19–21: podsumowanie i bibliografia. Status PLANNED/UNVERIFIED, bez renumeracji etapów.


**PDF20 (24 strony):** [audyt OmniE2E](docs/audits/PDF20_OMNIE2E_SEMANTIC_REVIEW_2026-10-16.md). Nowe wymagania: SwiftUI, Jetpack Compose, Rust FFI, natywne buildy Xcode/Gradle, Playwright/Appium, bezpieczne self-healing E2E, handoff OmniDev → OmniUX → OmniE2E. Etapy 0.46–0.47, 0.50–0.55, 0.63, 0.67, 0.98. PLANNED/UNVERIFIED.


## PDF22 — OmniWeb / OmniScrape (19 stron, 2026-10-16)

Audyt: `docs/audits/PDF22_OMNIWEB_SEMANTIC_REVIEW_2026-10-16.md`. S. 1–14: Zenoh/SHM, CRDT, OTLP, MCP, OAuth/ZKP, Firecracker/eBPF, L402, RAG/ASI, PQC/Loihi, OmniDev, OmniUX, OmniE2E, OmniForge — przeważnie powtórzenia. **S. 14–17: nowy rój OmniWeb**: `NawigatorAntyBot_Wasm`, `EkstraktorSemantyczny_DOM`, `InzynierWWW_FullStack`; autoryzowana nawigacja przeglądarką, ekstrakcja DOM do JSON/Markdown z walidacją schematu, generator aplikacji WWW/REST API, opcjonalne kontrolowane rozliczenia L402. Etapy 0.39, 0.43, 0.49, 0.55, **0.60 (OmniWeb)**, 0.63, 0.67, 0.98; luki zapisane w `missing_capabilities_v11.json`. S. 17: pięć rojów, nie pięć produktów Builder; nadal dziesięć wariantów Agents Builder. S. 18–19: bibliografia. Funkcje PDF zwracają stałe HTML/JSON/URL i drukują komunikat o zakupie proxy — brak prawdziwej implementacji. Obchodzenie CAPTCHA i zabezpieczeń bez autoryzacji nie jest kryterium odbioru. Status PLANNED_OR_UNVERIFIED; liczba etapów bez zmian.


## PDF23–PDF24 — synchronizacja wymagań (2026-10-16)

**PDF23, s. 15–17: OmniStudio i OmniDoc.** Rój generowania multimediów (graf renderingu obrazu/wideo/audio, synchronizacja, zgody i licencje) oraz generator dokumentacji (rzeczywista kompilacja Typst do PDF, wersjonowane szablony, kontrola artefaktów). Wymagane wykonawcze adaptery, typed handoff, budżety, sandbox, audyt i testy E2E. Przykład zwraca stałe nazwy MP4/WAV/PDF — nie dowodzi implementacji. Mapowanie: 0.43, 0.46–0.47, 0.49–0.50, 0.55, 0.63, 0.67, 0.98. Audyt: `docs/audits/PDF23_SEMANTIC_REVIEW_2026-10-16.md`.

**PDF24, s. 16–18: OmniGov, OmniFin, OmniPhys.** OmniGov: egzekwowany przed płatnością strażnik budżetu, odwołanie uprawnień L402/OAuth, fail-closed policy i niezależny audyt. OmniFin: pomiar kosztów tokenów, płatności i CPU oraz rozliczalne ROI. OmniPhys: uwierzytelnione urządzenia IoT/edge, interlock, watchdog, bezpieczny stan awaryjny i hardware-in-loop. Przykładowy kod tylko drukuje komunikaty i zwraca stały raport; brak dowodów odcięcia portfela, księgowania lub fizycznego sterowania. Mapowanie: 0.38–0.39, 0.43, 0.49, 0.55, 0.63, 0.67, 0.98. Audyt: `docs/audits/PDF24_SEMANTIC_REVIEW_2026-10-16.md`.

**Status:** PLANNED_OR_UNVERIFIED. Nowe roje nie zwiększają liczby 10 produktów Agents Builder; nie renumerować 215 etapów. Nie zaliczać prognoz, benchmarków bez pomiarów ani przykładowego kodu jako wdrożenia.


### PDF25–PDF26: OmniSec, OmniCog, OmniEvolve, OmniSense
Wymagania przypisane do istniejących etapów 0.38–0.41, 0.43, 0.49, 0.55, 0.63, 0.67 i 0.98. Audyt: `docs/audits/PDF25_26_SEMANTIC_REVIEW_2026-10-16.md`. Status: PLANNED_OR_UNVERIFIED. Przykłady z PDF nie są dowodem wykonania; zachowano 10 produktów Agents Builder i numerację etapów.


### PDF27–PDF29 (2026-10-16)
Uzgodniono wymagania: PDF27 OmniSense (0.67); PDF28 OmniArbitrage (0.49), OmniContract/OmniChaos (0.43), OmniStrategy/OmniGraph (0.67); PDF29 OmniVerify/OmniHomeostasis (0.55), OmniCausal (0.98), OmniSleep/OmniPrune (0.67), OmniLatent/OmniAdaptive (0.39/0.63). Szczegółowe kryteria odbioru: ROADMAP.md. Status UNVERIFIED; przykłady z PDF nie stanowią implementacji.


### PDF30–PDF33 — synchronizacja wymagań (2026-10-16)
PDF30: aktywna inferencja, HDC i fraktalny rój → 0.67/0.98. PDF31: migracja wiedzy, proof-carrying code, energia i alignment → 0.55/0.63/0.98. PDF32: Agents Builder Cloud/Windows/macOS/Android/iOS × Code/NoCode, weryfikacja wizualna → 0.50–0.55. PDF33: OmniMath, Lean 4, egg/e-graph, Z3/cvc5 i ograniczone przeszukiwanie dowodów → 0.55/0.67. Status PLANNED_OR_UNVERIFIED. Przykłady PDF33 nie wykonują SMT ani Lean; kryteria i luki w ROADMAP.md.


## PDF34–PDF36 — audyt wymagań i faz M0–M6 (2026-10-16)
Źródło potwierdzone: raport „Kompleksowy Raport Badawczo-Architektoniczny: Koncepcja i Mapa Drogowa Systemu OCAF oraz JARVIS OMEGA INFINITY”, 12 stron (PDF34–36 w pakiecie użytkownika; przypisanie numeru konkretnego pliku wymaga osobnej identyfikacji). Wszystkie poniższe wymagania: PLANNED_OR_UNVERIFIED, bez dowodu testów repo/CI.
- s. 2–3, M0: typowane SDK MCP rmcp/pmcp, stateless HTTP/streaming, control plane, zgodność protokołu, autoryzacja fail-closed. Kryterium: testy zgodności i obciążenie 500 równoległych żądań, bez założenia gwarantowanej przepustowości.
- s. 3–4, M1: Zenoh pub/sub i query/reply, SHM/ZBytes zero-copy lokalnie, lifetime/ACL pamięci i fallback dla hostów zdalnych; CRDT Automerge/Yrs, offline-first, konflikt/recovery. Kryterium: wielohostowe testy izolacji, replay, trwałości i benchmark opóźnień; 10 000 agentów to cel eksperymentalny.
- s. 4–5, M4: Tree-sitter/type-sitter do indeksacji kodu oraz semantyczny ranking narzędzi, redukcja JSON Schema; wymagana poprawność wyboru, negatywne testy prompt injection i niezależne ACL. KPI ≥95% trafności i ≥40% redukcji kontekstu to hipotezy do pomiaru.
- s. 5–6, M2: CRDT, L402/macaroons/caveats, opcjonalne Lightning i ZKP Arkworks/Halo2. Wymagane wiążące dowody autoryzacji, replay protection, ledger/idempotencja, tenant isolation, audyt; żadnych automatycznych realnych płatności bez jawnego zatwierdzenia. Testy offline i negatywne.
- s. 6–7, M3: Firecracker/KVM/Jailer/seccomp, WASI w gościu, broker uprawnień, Aya/eBPF, detekcja i odtwarzanie po kompromitacji. Brama 0.39; testy sprzętowe na realnym hoście, brak gwarancji izolacji <10 µs bez pomiarów.
- s. 7–8, M4: PQC FIPS203 ML-KEM, FIPS204 ML-DSA, hybrydowy handshake X25519+ML-KEM, zarządzanie kluczami, downgrade resistance, interoperacyjność i rotacja. Wymagane testy kryptograficzne i integracyjne.
- s. 8–10, M5–M6: neuromorficzny SNN/Loihi2 Hardware-in-the-Loop, fallback CPU/GPU, pomiar energy-delay product, sprzężenie PQC. Brak dostępu do sprzętu = R&D, nie INTEGRATED; KPI 90% EDP i <1 µs pełnego routingu/PQC niezweryfikowane.
- s. 8–9: finansowanie FENG/NCBR/PARP/GAPR i ryzyka dostępności sprzętu są hipotezami biznesowymi, nie dowodami przyznania dotacji.
Mapowanie: M0 do MCP/tool gateway; M1 do transport/cluster/registry; M2 do auth/ops; M3 do 0.39 i recovery; M4 do narzędzi/AST i security; M5–M6 do późnych etapów R&D. Zachować istniejące numery etapów i bramy 0.38–0.41. Liczby i twierdzenia marketingowe PDF nie są statusami implementacji.


## PDF34–PDF36 — numerowane etapy główne 216–222 (2026-10-16)
- **216** — MCP stateless control plane — PLANNED
- **217** — Zenoh SHM i CRDT — PLANNED
- **218** — L402 Macaroons ZKP — PLANNED
- **219** — Firecracker KVM Aya eBPF — PLANNED
- **220** — Tree-sitter i hybrydowe PQC — PLANNED
- **221** — Neuromorficzne SNN Loihi 2 — PLANNED
- **222** — Kwantowo-neuromorficzne HIL PQC — PLANNED
Kryteria odbioru: ROADMAP.md. Historyczne etapy bez renumeracji. Łącznie 222 etapy główne.


## PDF37–38 — etapy 223–230 (2026-10-16)
### Etap 223 — PDF37: protokoły negocjacji zasobów i dynamic throttling
**Status: PLANNED.** Licytacje GPU/API, cost-latency-accuracy, polityki budżetowe i deterministyczny arbitraż. Odbiór: rzeczywista implementacja, testy pozytywne i negatywne, CI/SHA, właściwa infrastruktura; przykłady z PDF nie są dowodem implementacji.

### Etap 224 — PDF37: telemetryczny routing modeli i kontrolowane RLAIF
**Status: PLANNED.** Multi-armed bandits, epsilon-greedy, pomiar jakości, opóźnienia i regresji, bezpieczny feedback. Odbiór: rzeczywista implementacja, testy pozytywne i negatywne, CI/SHA, właściwa infrastruktura; przykłady z PDF nie są dowodem implementacji.

### Etap 225 — PDF37: ważony konsensus ekspertów i ograniczona samonaprawa
**Status: PLANNED.** Głosowanie domenowe, konflikty danych, limit prób, fail-closed i ślad decyzji. Odbiór: rzeczywista implementacja, testy pozytywne i negatywne, CI/SHA, właściwa infrastruktura; przykłady z PDF nie są dowodem implementacji.

### Etap 226 — PDF37: Actor-Critic, grounding i weryfikacja faktów
**Status: PLANNED.** Weryfikator niezależny od generatora, provenance, dowody, abstencja, bounded reasoning. Odbiór: rzeczywista implementacja, testy pozytywne i negatywne, CI/SHA, właściwa infrastruktura; przykłady z PDF nie są dowodem implementacji.

### Etap 227 — PDF37: DAG delegowania i typowane tool calling
**Status: PLANNED.** Graf zależności, semantic routing, least privilege, idempotencja, audyt wywołań. Odbiór: rzeczywista implementacja, testy pozytywne i negatywne, CI/SHA, właściwa infrastruktura; przykłady z PDF nie są dowodem implementacji.

### Etap 228 — PDF38: wielotransportowa warstwa komunikacji urządzeń
**Status: PLANNED.** Adaptery BLE, NFC, UWB, Wi-Fi Direct, IrDA, Li-Fi/OCC/FSO z capability negotiation i fallback. Odbiór: rzeczywista implementacja, testy pozytywne i negatywne, CI/SHA, właściwa infrastruktura; przykłady z PDF nie są dowodem implementacji.

### Etap 229 — PDF38: brama IoT/LPWAN i przemysłowe protokoły
**Status: PLANNED.** LoRaWAN, NB-IoT, LTE-M, Thread/Matter, Zigbee, 6LoWPAN, izolacja i bezpieczne provisioning. Odbiór: rzeczywista implementacja, testy pozytywne i negatywne, CI/SHA, właściwa infrastruktura; przykłady z PDF nie są dowodem implementacji.

### Etap 230 — PDF38: transport mobilny, lokalizacja i źródła RSS
**Status: PLANNED.** Integracje 4G/5G, GNSS, MSISDN bez ujawniania danych, RSS, autoryzacja i ograniczenia platformowe. Odbiór: rzeczywista implementacja, testy pozytywne i negatywne, CI/SHA, właściwa infrastruktura; przykłady z PDF nie są dowodem implementacji.


## PDF37–PDF40 — etapy 223–231 (2026-10-16)

### Etap 223 — Taksonomia systemów agentowych i frameworków
**Status: PLANNED.** PDF38 s.1–58; katalog implementacji i frameworków, klasyfikacja, ocena dojrzałości, nie utożsamiać referencji z integracją. Wymagane: typowane API Rust, fail-closed authorization, izolacja tenantów, testy negatywne i integracyjne, dowód CI oraz walidacja na właściwym środowisku. Przykłady PDF nie stanowią implementacji.

### Etap 224 — Registry agent skills i 110+ umiejętności miękkich
**Status: PLANNED.** PDF39 s.1–51; typowane kontrakty Rust, wersjonowanie, walidacja, sandbox, rejestr umiejętności i testy. Wymagane: typowane API Rust, fail-closed authorization, izolacja tenantów, testy negatywne i integracyjne, dowód CI oraz walidacja na właściwym środowisku. Przykłady PDF nie stanowią implementacji.

### Etap 225 — Negocjacje wieloagentowe i aukcje zasobów
**Status: PLANNED.** PDF39 s.3; licytacja GPU/API, fairness, quota, cost-latency-accuracy, dynamic throttling. Wymagane: typowane API Rust, fail-closed authorization, izolacja tenantów, testy negatywne i integracyjne, dowód CI oraz walidacja na właściwym środowisku. Przykłady PDF nie stanowią implementacji.

### Etap 226 — Telemetry-driven routing, bandyty i feedback
**Status: PLANNED.** PDF39 s.4–5; epsilon-greedy, metryki, RLHF/RLAIF z nadzorem i rollback. Wymagane: typowane API Rust, fail-closed authorization, izolacja tenantów, testy negatywne i integracyjne, dowód CI oraz walidacja na właściwym środowisku. Przykłady PDF nie stanowią implementacji.

### Etap 227 — Konsensus ekspertów, samonaprawa i grounding
**Status: PLANNED.** PDF39 s.5–9; weighted vote, bounded retries, actor-critic, dowody źródłowe. Wymagane: typowane API Rust, fail-closed authorization, izolacja tenantów, testy negatywne i integracyjne, dowód CI oraz walidacja na właściwym środowisku. Przykłady PDF nie stanowią implementacji.

### Etap 228 — Bezpieczna dekompozycja DAG i delegacja narzędzi
**Status: PLANNED.** PDF39 s.9–10; typed graph, semantic routing, uprawnienia i real tool calls. Wymagane: typowane API Rust, fail-closed authorization, izolacja tenantów, testy negatywne i integracyjne, dowód CI oraz walidacja na właściwym środowisku. Przykłady PDF nie stanowią implementacji.

### Etap 229 — Polimorficzny transport i discovery łączności
**Status: PLANNED.** PDF40 s.1–23; IrDA, BLE, NFC, UWB, Wi-Fi, Li-Fi, FSO, LPWAN; capability negotiation i zgodność urządzeń. Wymagane: typowane API Rust, fail-closed authorization, izolacja tenantów, testy negatywne i integracyjne, dowód CI oraz walidacja na właściwym środowisku. Przykłady PDF nie stanowią implementacji.

### Etap 230 — Mobilny i przemysłowy edge networking
**Status: PLANNED.** PDF40 s.12–24; LTE/5G, NB-IoT, LoRaWAN, Thread/Matter, telemetry, security, ograniczenia radiowe. Wymagane: typowane API Rust, fail-closed authorization, izolacja tenantów, testy negatywne i integracyjne, dowód CI oraz walidacja na właściwym środowisku. Przykłady PDF nie stanowią implementacji.

### Etap 231 — Interoperacyjność komunikacji i adapterów OCAF
**Status: PLANNED.** PDF40 s.24–118; katalog protokołów, polityka kompatybilności, testy hardware-in-loop, brak domniemanej integracji. Wymagane: typowane API Rust, fail-closed authorization, izolacja tenantów, testy negatywne i integracyjne, dowód CI oraz walidacja na właściwym środowisku. Przykłady PDF nie stanowią implementacji.

PDF37: zakres MCP/Zenoh/PQC/neuromorfika przypisany do istniejących 216–222 bez duplikacji.


## Rozszerzenie z dokumentów PDF 45–47 (2026-10-16)

| Etap / numer | Opis | Status |
| :-- | :-- | :-- |
| **247** | **Zenoh SHM zero-copy i typowane bufory** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **248** | **CRDT Automerge/Yrs i synchronizacja pamięci** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **249** | **OpenTelemetry: propagacja kontekstu Tokio** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **250** | **MCP schema fusion: Tree-sitter CST/AST** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **251** | **OAuth/Macaroons i weryfikowalne uprawnienia ZKP** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **252** | **Firecracker eBPF runtime threat detection** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **253** | **Ekonomia M2M L402 i offline payment queue** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **254** | **Hybrydowe PQC ML-KEM/ML-DSA** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **255** | **OmniDev: wieloagentowa inżynieria kodu** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **256** | **OmniUX: Figma MCP i visual regression** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **257** | **OmniE2E: Android/iOS i testy GUI** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **258** | **OmniForge: ECS i headless Vulkan** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **259** | **OmniCog: neuro-symboliczne wnioskowanie** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **260** | **OmniContext: kontekst i pamięć grafowa** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **261** | **OmniAgent Builders: platformy Code/NoCode** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **262** | **Kontrola kosztów i budżetów autonomii** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **263** | **Formalne kontrakty bezpieczeństwa narzędzi** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **264** | **Weryfikacja wydajności i benchmarków** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **265** | **Integracja agentów specjalistycznych** — wymaganie z PDF 45; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **266** | **Ontologie i reprezentacja wiedzy AI** — wymaganie z PDF 46; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **267** | **Logika formalna i dowody SAT/SMT** — wymaganie z PDF 46; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **268** | **Algorytmy wyszukiwania i planowania** — wymaganie z PDF 46; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **269** | **Probabilistyka i niepewność w agentach** — wymaganie z PDF 46; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **270** | **Ewaluacja generalizacji i optymalizacji** — wymaganie z PDF 46; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **271** | **OmniSeek-Local: lokalni agenci i bounded DAG** — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **272** | **OmniFlow-Pieces: workflow Code/NoCode** — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **273** | **OmniDroid-GUI: Android Accessibility i HITL** — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **274** | **OmniEval-Lab: ewaluacja trajektorii** — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **275** | **OmniFleet-C2: rejestr i hot reload Skills** — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **276** | **OmniVoice-Duplex i OmniCinema** — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |
| **277** | **OmniAgro-KG: domenowy graf wiedzy** — wymaganie z PDF 47; implementacja Rust, kontrakty, izolacja tenantów, testy i dowody integracyjne; bez uznawania przykładów dokumentacyjnych za gotowy kod. | **DO ZROBIENIA** |


### Dokumenty 48–50 — kontrola duplikatów (2026-10-16)

Ponownie przesłane materiały: **48 = Notes_261014_204557.pdf (127 stron)**, **49 = Dokument bez nazwy.PDF (14 stron)**, **50 = Architektura OmniContext Agent Framework (OCAF) w ..(1).PDF (61 stron)**. Są to te same dokumenty merytoryczne, które uprzednio zarejestrowano jako PDF 46, 47 i 45. **Nie dopisano nowych etapów**: ich wymagania zostały już wpisane pod identyfikatorami **247–277** (PDF 45: 247–265; PDF 46: 266–270; PDF 47: 271–277). Kolejny wolny identyfikator pozostaje **278**. Ponowne przesłanie dokumentu nie oznacza nowej funkcjonalności ani wykonania testów.


## Etapy 301–320 — nowe dokumenty (2026-10-16)

| **301** | OmniNous Atropos RL i CNA (PDF51 s.1,4-6) | **PLANNED** |
| **302** | OmniHoncho IdentityMind i Groupchat Arbiter (PDF51 s.1,6-7) | **PLANNED** |
| **303** | OmniCloudShield AntiStealer (PDF51 s.2,7-8) | **PLANNED** |
| **304** | OmniSentry VoIP Guard (PDF51 s.2,7-8) | **PLANNED** |
| **305** | OmniMask IdentityRelay (PDF51 s.2,9-10) | **PLANNED** |
| **306** | OmniTelecom eSIM/PBX (PDF51 s.2,9-10) | **PLANNED** |
| **307** | OmniVox BenchmarkRouter (PDF51 s.2-3,10-11) | **PLANNED** |
| **308** | OmniGrant Architect i ERP (PDF51 s.3,11-12) | **PLANNED** |
| **309** | OmniPlasmic AST Compiler (PDF51 s.3,11-12) | **PLANNED** |
| **310** | Zenoh SHM + Automerge/Yrs + embedded (PDF52 s.1; PDF53 s.1-2) | **PLANNED** |
| **311** | OpenTelemetry produkcyjny pipeline (PDF52 s.1; PDF53 s.1-3) | **PLANNED** |
| **312** | CoALA UALA Titans memory (PDF52 s.1,5-6; PDF53 s.1,3) | **PLANNED** |
| **313** | Red Queen Gödel Machine i type-sitter (PDF52 s.2,7-8; PDF53 s.1,3-4) | **PLANNED** |
| **314** | Loihi 2 Sigma-Delta SDNN (PDF52 s.2,8-9; PDF53 s.1,4) | **PLANNED** |
| **315** | mlkem-native PQC integration (PDF52 s.2; PDF53 s.1,4) | **PLANNED** |
| **316** | In-guest Aya eBPF EDR (PDF52 s.2-3,10-11; PDF53 s.1-2,4,7-9) | **PLANNED** |
| **317** | Cost Exhaustion i Action Tool Verification (PDF52 s.3,10-11; PDF53 s.2,4-5,10-11) | **PLANNED** |
| **318** | Disposable Apps Lifecycle i Energy Scheduler (PDF52 s.3; PDF53 s.2,4-5,10-11) | **PLANNED** |
| **319** | L402 Macaroon settlement and caveats (PDF52 s.3,11-12; PDF53 s.2,4-5,11-12) | **PLANNED** |
| **320** | Sovereign Cargo workspace integration (PDF53 s.2,5-13) | **PLANNED** |


<!-- OCAF_321_333_SYNC -->
| Etap | Zakres | Status |
|---|---|---|
| **321** | Architektury poznawcze i neuro-symboliczne | **PLANNED** |
| **322** | Pamięć agentowa i konsolidacja wiedzy | **PLANNED** |
| **323** | Symulacje społeczne i Theory of Mind | **PLANNED** |
| **324** | Samooptymalizacja i autonomiczne badania | **PLANNED** |
| **325** | Inżynieria agentowa i ewaluacje | **PLANNED** |
| **326** | DGM/RQGM: kontrolowana ewolucja | **PLANNED** |
| **327** | CoALA/Titans, HDC i bitemporalne CRDT | **PLANNED** |
| **328** | GoT/AB-MCTS/JEPA oraz AST/SMT | **PLANNED** |
| **329** | Firecracker, eBPF i MCP/L402 | **PLANNED** |
| **330** | OmniModelScout: walidacja GGUF | **PLANNED** |
| **331** | TechRecon i Strategist | **PLANNED** |
| **332** | AppForge 66 i GameBuilder | **PLANNED** |
| **333** | Audyt implementacji DGM i konsolidacji 606 repozytoriów | **PLANNED** |


### Etap 334 — zweryfikowane laboratoryjne CI, bez produkcyjnego odbioru (2026-10-29)

- [SASOS katalog 555, capability, adapter HTTPS/Vault, Z3 i testy pamięci — PASS](https://github.com/mojealterego/ocaf-runtime/actions/runs/40941557248), SHA `261f8ed7a268eff4a771bce77b2ef6efbc7695bc`.
- [Zenoh 1.10.1, dwa procesy Linux, rzeczywisty SHM i fallback — PASS](https://github.com/mojealterego/ocaf-runtime/actions/runs/40942073444), SHA `d52c9d9f18c4363eebedf266fdee0e079578ae2a`. To nie zero-copy między hostami ani integracja Rust OMAt.
- Katalog 555 pozostaje `CATALOG_ONLY`; **0 potwierdzonych aktywnych backendów**. Operator-owned IdP/Vault, docelowy Firecracker i pełny runtime E2E niezatwierdzone. OSTc odłożone zgodnie z decyzją operatora, bez zmiany dotychczasowego fail-closed.

Szczegóły: [deploy/stage334/README.md](deploy/stage334/README.md).

### Argon PDF-1.15–PDF-1.54 (40 wymagań) — macierz z 2026-10-30

- [Tabela 40/40 w głównej Roadmapie](ROADMAP.md#argon--źródłowe-punkty-pdf-115pdf-154-4040-2026-10-30), [manifest powiązań z etapami](docs/roadmap/argon_pdf_115_154_manifest.json), [audyt i braki](docs/audits/ARGON_115_154_RECONCILIATION_2026-10-30.md).
- `src/governance_journal.rs`: trwała idempotencja i audit; `src/retrieval_quality.rs`: leksykalny BM25 i Recall@k/MRR. **Status: [CI testów lokalnych — PASS](https://github.com/mojealterego/ocaf-runtime/actions/runs/40989435834), E2E nadal otwarte.** Binariów nie pakowano. PDF-1.52 ma oznaczenie **ZROBIONE — lokalny ewaluator jakości** (testy Recall@k/MRR), ale nie oznacza gotowości pełnego systemu retrieval. Nie zmieniono statusów kanonicznych etapów bez dowodów.
