# Rejestr agentów i komponentów runtime

**Wersja rejestru:** 1.0 • **Stan:** roboczy, weryfikowalny • **Data:** 2026-10-21

Ten rejestr opisuje każdy utrzymywany element systemu według rozdziału z dokumentacji budowy. Wpis `IMPLEMENTED_VERIFIED` wymaga kodu, artefaktu, identyfikatora commita, testów i dowodu w `docs/evidence/`. Brak któregoś z tych dowodów pozostaje `PARTIAL` albo `PLANNED`.

## Komponenty platformowe

Pełny kontrakt infrastruktury: [`docs/infrastructure/system-infrastructure-v1.json`](infrastructure/system-infrastructure-v1.json).
Dowody dla statusów agentów i rojów przechodzą przez [`EVIDENCE_PIPELINE.md`](EVIDENCE_PIPELINE.md).

| ID | Komponent | Odpowiedzialność | Stan | Warunek przejścia do ACTIVE |
|---|---|---|---|---|
| OCAF-RUNTIME | OCAF Runtime | cykl życia, kontrakty, wykonanie i integracja agentów | PARTIAL | kompletna paczka agenta, testy E2E, podpisany artefakt |
| OMAS-SERVER | OMAs | osobny serwer agentów i rejestr/admisja | PARTIAL | niezależny proces/protokół, authn/authz, testy wielodzierżawne |
| OMAT-SERVER | OMAt | osobny serwer narzędzi i rejestr uprawnień | PARTIAL | katalog narzędzi, zakresy uprawnień, izolowany wykonawca |
| OSTC-CONTROL | OSTc | przyszłe szyfrowanie dostępu do źródeł | DISABLED | zatwierdzona specyfikacja kryptograficzna i dowód integracji |
| ZENOH-TRANSPORT | Zenoh | transport komunikatów i synchronizacji | PARTIAL | dowód kontraktu transportowego z OMAs i OMAt |

**Reguła bezpieczeństwa:** przy `OSTc-CONTROL = DISABLED` dostęp agenta do pliku źródłowego jest wyłączony, a `ACTIVE` jest odrzucane także dla fałszywych potwierdzeń gotowości. Ta reguła jest obowiązkowa dla każdego wpisu agenta.

## Agenci i produkty Agents Builder

### Flota 111 osobnych agentów AI

Flota jest zapisana jako 111 niezależnych pakietów w
[`AGENT-FLEET-111`](<../OCAF Agents MOJEALTEREGO/AGENT-FLEET-111/>). Każdy pakiet ma własną unikalną nazwę oryginalną (`display_name`) oraz stabilny identyfikator techniczny `OCAF-AGENT-NNN`. Nazwa jest częścią manifestu, konfiguracji, punktu wejścia, dokumentacji i nazwy pakietu Cargo.

Każdy pakiet ma własną tożsamość, kod, manifest, kontrakt modelu, namespace pamięci,
politykę narzędzi, test bezpieczeństwa, instrukcję, kontrakt instalatora i
rekord certyfikacyjny. `model_backend`, OMAs i OMAt są jeszcze
`UNASSIGNED`, dlatego rejestr nie udaje uruchomionych procesów: aktywnych
runtime jest `0`. Kontrolę wykonuje
[`ocaf_agent_fleet_supervisor.py`](../scripts/ocaf_agent_fleet_supervisor.py).

### Flota 33 kompletnych rojów AI

Każdy rój ma własny manifest, skład, kontrakt etapowania i handoff, healthcheck,
test kontraktu oraz rekord certyfikacyjny. Rejestr: [SWARM_REGISTRY_33.md](agents/SWARM_REGISTRY_33.md),
manifest maszynowy: [swarm_registry_33.json](agents/swarm_registry_33.json),
pakiety: [SWARM-FLEET-33](<../OCAF Agents MOJEALTEREGO/SWARM-FLEET-33/>).

Poniższe dziesięć pozycji to warianty produktu Buildera (Cloud/Desktop/Mobile), nie dziesięć gotowych agentów runtime. Ich status jest synchronizowany z [`docs/roadmap/agents_builder_products_v11.json`](roadmap/agents_builder_products_v11.json).

Gate instalatorów i wymagane artefakty dla tych wariantów: [`BUILDER_INSTALLER_MATRIX.md`](BUILDER_INSTALLER_MATRIX.md).

| ID | Produkt | Platforma / tryb | Stan | Artefakt instalacyjny | Dowód |
|---|---|---|---|---|---|
| AB-CLOUD-CODE | Agents Builder Chmurowy Code | Cloud / Code | NOT_IMPLEMENTED | brak | brak |
| AB-CLOUD-NOCODE | Agents Builder Chmurowy NoCode | Cloud / NoCode | NOT_IMPLEMENTED | brak | brak |
| AB-WINDOWS-CODE | Agents Builder Desktopowy Windows Code | Windows / Code | NOT_IMPLEMENTED | brak | brak |
| AB-WINDOWS-NOCODE | Agents Builder Desktopowy Windows NoCode | Windows / NoCode | NOT_IMPLEMENTED | brak | brak |
| AB-MACOS-CODE | Agents Builder Desktopowy macOS Code | macOS / Code | NOT_IMPLEMENTED | brak | brak |
| AB-MACOS-NOCODE | Agents Builder Desktopowy macOS NoCode | macOS / NoCode | NOT_IMPLEMENTED | brak | brak |
| AB-ANDROID-CODE | Agents Builder Android Code | Android / Code | NOT_IMPLEMENTED | brak | brak |
| AB-ANDROID-NOCODE | Agents Builder Android NoCode | Android / NoCode | NOT_IMPLEMENTED | brak | brak |
| AB-IOS-CODE | Agents Builder iOS Code | iOS / Code | NOT_IMPLEMENTED | brak | brak |
| AB-IOS-NOCODE | Agents Builder iOS NoCode | iOS / NoCode | NOT_IMPLEMENTED | brak | brak |

## Rejestr referencyjnego agenta

`OCAF-AGENT-001` jest referencyjnym pakietem zgodności dla układu katalogów z instrukcji użytkownika. Wpis staje się `IMPLEMENTED_VERIFIED` dopiero po wskazaniu SHA-256 paczki, commita budowy i wyników testów. OSTc pozostaje wyłączone, więc jego konfiguracja nie może osiągnąć `ACTIVE`.

- Manifest i konfiguracja: `OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/config/agent.toml`
- Dokumentacja: `OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/docs/`
- Testy: `OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/tests/`
- Rejestr certyfikatu: `OCAF Agents MOJEALTEREGO/OCAF-AGENT-001/certification/CERTIFICATE_RECORD.md`

## Kryteria wpisu

1. Unikalny `agent_id`, właściciel, wersja i manifest.
2. Wejście/wyjście oraz zakres uprawnień narzędzi OMAt.
3. Tożsamość/protokół OMAs i transport Zenoh.
4. Limity zasobów, izolacja, obsługa błędów, logowanie i retencja.
5. Testy jednostkowe, integracyjne, awarii i bezpieczeństwa.
6. Artefakt, digest, commit budowy i dowód rejestracji.
7. Status cyklu życia zgodny z bramką OSTc.

Opis wszystkich 33 grup operacyjnych i ich zasad handoff znajduje się w
[`docs/agents/AGENT_CATALOG_33.md`](agents/AGENT_CATALOG_33.md).

Organizacyjny rejestr bieżącej dyspozycji **33 rojów / 111 osobnych agentów** znajduje się w
[`agents/AGENT_REGISTRY_111.md`](agents/AGENT_REGISTRY_111.md), a wersja maszynowa w
[`agents/agent_registry_111.json`](agents/agent_registry_111.json). Te wpisy mają
status `NOT_DEPLOYED` i nie są dowodem aktywnych agentów runtime. Fizyczne,
odrębne katalogi agentów znajdują się w
[`OCAF Agents MOJEALTEREGO/AGENT-FLEET-111`](<../OCAF Agents MOJEALTEREGO/AGENT-FLEET-111/>).
