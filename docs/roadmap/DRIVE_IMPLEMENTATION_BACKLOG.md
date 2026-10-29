# Drive: lokalizacja wymagań i pozostałe prace

Odniesienie: `0ab23d5`. Statusy oznaczają stan wymagania, nie zamknięcie całego etapu. `PROPOSED` wymaga uzgodnienia semantycznego; `UNMAPPED` oznacza brak bezpiecznego przypisania. Żadna pozycja nie została oznaczona produkcyjnym DONE.

## Kolejność prac

1. Dokończyć rzeczywisty E2E OAuth/Vault/provider i integrację istniejących capabilities/adapters (334).
2. Zweryfikować pozostałe kontrakty sandbox/SHM/CRDT oraz modelowanie i pamięć.
3. Wdrożyć wspólne IR i roundtrip Code/NoCode (0.50), następnie edytory (0.51).
4. Dostarczać osobno dziesięć produktów builderów; pakowanie i akceptacja 0.55.
5. Rozstrzygnąć zakres Open CASCADE i przypisania PROPOSED przed importem kodu domenowego.

## Rejestr

| ID / źródło | Wymaganie | Kanoniczne punkty | Status / mapowanie | Pozostało |
| --- | --- | --- | --- | --- |
| DRIVE-001 oauth: dispatcher | 555 backendów | 334 | PARTIAL / SEMANTIC | Bindingi i sekrety konkretnych providerów; realny E2E; następne klasy backendów; OSTc dla ścieżek uprzywilejowanych |
| DRIVE-002 oauth: 0.44 | OAuth OIDC PKCE MFA Vault | 0.44 | PARTIAL / SEMANTIC | Rzeczywisty IdP/Vault/refresh/revoke E2E; JWKS rotation; akceptacja docelowej infrastruktury |
| DRIVE-003 oauth: 0.45 | Capabilities | 1.46, 1.50, 1.51, 334 | PARTIAL / SEMANTIC | Audyt trwałego revocation/replay i integracji w każdej ścieżce wykonania |
| DRIVE-004 oauth: 0.46 | WASI Firecracker | 0.39, 334 | LAB_VERIFIED / SEMANTIC | Testy na hoście operatora, powiązanie z dyspozytorem i OSTc, odbiór produkcyjny |
| DRIVE-005 oauth: 0.47 | Zenoh SHM CRDT | 0.46, 1.15, 1.18, 1.19, 1.55, 1.57 | PARTIAL / SEMANTIC | Produkcyjna ścieżka OMAt SHM; typed memory; CRDT convergence/offline partitions; benchmarki |
| DRIVE-006 oauth: 0.48 | BMC LTN | 334 | PARTIAL / SEMANTIC | Model rzeczywistego kodu i granice dowodu; adapter ImandraX; walidacja LTN na realnych wejściach |
| DRIVE-007 oauth: 0.49 | Titans NICE RQGM | 1.27, 1.29, 1.30, 1.67, 1.70, 334 | PARTIAL / SEMANTIC | Rzeczywiste modele i wagi; korpus jakości; stabilność; formalne granice mutacji i rollback |
| DRIVE-008 builder: 0.50 | Wspólne IR Code/NoCode | 0.50 | NOT_IMPLEMENTED / EXACT | Typed DAG, ograniczony DSL, deterministyczny roundtrip i migracje |
| DRIVE-009 builder: 0.51 | Edytory Code i NoCode | 0.51 | NOT_IMPLEMENTED / EXACT | Diagnostyka Code, canvas NoCode, zgodność reprezentacji |
| DRIVE-010 builder: 0.52 | Cloud Code i NoCode | 0.52 | NOT_IMPLEMENTED / EXACT | Dwa produkty przeglądarkowe z izolacją tenantów i E2E |
| DRIVE-011 builder: 0.53 | Windows/macOS Code i NoCode | 0.53 | NOT_IMPLEMENTED / EXACT | Cztery aplikacje, rzeczywiste buildy i GUI/offline E2E |
| DRIVE-012 builder: 0.54 | Android/iOS Code i NoCode | 0.54 | NOT_IMPLEMENTED / EXACT | Cztery aplikacje, buildy platformowe i testy na urządzeniach/symulatorach |
| DRIVE-013 builder: 0.55 | Pakowanie i weryfikacja builderów | 0.55 | NOT_IMPLEMENTED / EXACT | Manifesty, sandbox preview, podpisy, kontrola uprawnień i wydania |
| DRIVE-014 builder: design/i18n | Atelier Sovereign i polski język kanoniczny | 0.51, 0.52, 0.53, 0.54 | NOT_IMPLEMENTED / SEMANTIC | Design tokens, katalog PL, fallback locale, dostępność i testy UI |
| DRIVE-015 roadmap115: 1.15–1.22 | Referencje Label/GUID, transakcje, replay, persistence | do ustalenia | DECISION_REQUIRED / UNMAPPED | Ustalić czy warstwa dokumentów Open CASCADE należy do produktu; określić kontrakt i migrację bez drugiego runtime |
| DRIVE-016 roadmap115: 1.23–1.30 | Policy, approval, idempotency, tenant, koszty | 0.44, 1.46, 1.47 | REQUIRES_VERIFICATION / PROPOSED | Porównać każdy kontrakt z OMAs/OMAt; approval binding i audyt, limity kosztów |
| DRIVE-017 roadmap115: 1.31–1.38 | ContractObligations | 1.92, 1.93 | REQUIRES_VERIFICATION / PROPOSED | Ekstrakcja zobowiązań, terminy, evidence, role i interfejsy domenowe |
| DRIVE-018 roadmap115: 1.39–1.46 | ComplianceEvidence i SupplierDueDiligence | 1.92, 1.93 | REQUIRES_VERIFICATION / PROPOSED | Łańcuch dowodów, screening, limity zakupowe, autoryzacja i testy domenowe |
| DRIVE-019 roadmap115: 1.47–1.54 | Wyszukiwanie, orchestration, UX | 1.65, 1.94 | REQUIRES_VERIFICATION / PROPOSED | Porównać retrieval/router/UX z istniejącym runtime; rzeczywiste dane i ewaluacja |
| DRIVE-020 architecture155: 1.55–1.57 | Drzewo Label/GUID, undo/redo, recomputation | do ustalenia | DECISION_REQUIRED / UNMAPPED | Rozstrzygnąć zakres Open CASCADE i wspólne IR zamiast nadpisania etapów transportu |
| DRIVE-021 architecture155: 1.58–1.60 | Policy, event store i sterowniki persistence | 0.44, 1.46 | REQUIRES_VERIFICATION / PROPOSED | Sprawdzić atomowość i idempotencję, kontrakty approval oraz migracje danych |
| DRIVE-022 architecture155: 1.61–1.65 | Retrieval lexical/vector | 1.65 | REQUIRES_VERIFICATION / PROPOSED | Realne adaptery i ocena retrieval na korpusie |
| DRIVE-023 architecture155: 1.66–1.70 | Reasoning i self-consistency | 1.66, 1.69 | REQUIRES_VERIFICATION / PROPOSED | Ślady decyzji, ograniczenia kosztów i niezależna ocena jakości |
| DRIVE-024 architecture155: 1.71–1.75 | Router modeli | 1.94 | REQUIRES_VERIFICATION / PROPOSED | Wiązanie realnych providerów, profile kosztów/latencji i awarie |
| DRIVE-025 architecture155: 1.76–1.80 | Prompt DSL i koperty | 0.50, 0.51 | NOT_IMPLEMENTED / PROPOSED | Parser i typowanie DSL, walidacja oraz testy roundtrip |
| DRIVE-026 architecture155: 1.81–1.85 | GEM H-MoE OODA | 1.92, 1.94 | REQUIRES_VERIFICATION / PROPOSED | Zweryfikować realne wykonanie grafów, routing i zakończenie pętli |
| DRIVE-027 architecture155: 1.86–1.90 | Pakiety domenowe | 1.92, 1.93 | REQUIRES_VERIFICATION / PROPOSED | Implementacja kontraktów domenowych i akceptacja z danymi |
| DRIVE-028 architecture155: 1.91–1.99 | ClaimLedger i dowody | 1.66, 1.99 | REQUIRES_VERIFICATION / PROPOSED | Schemat provenance, evidence binding, niezależna ocena bez samocertyfikacji |
| DRIVE-029 architecture155: 2.00 | Unified kernel | 1.94 | REQUIRES_VERIFICATION / PROPOSED | Integracja obecnego runtime, kontrakty granic i E2E |
| DRIVE-030 architecture155: 2.01 | Zenoh transport | 0.46, 1.15 | PARTIAL / PROPOSED | Zintegrować z produkcyjną ścieżką OMAt i sprawdzić backpressure |
| DRIVE-031 architecture155: 2.02 | Model kolaboracji | 2.03 | REQUIRES_VERIFICATION / PROPOSED | Jawne metryki ewaluacji i bramki operatora |
| DRIVE-032 architecture155: 2.03 | Voice narrative firewall | 1.05 | REQUIRES_VERIFICATION / PROPOSED | Rzeczywisty pipeline audio, walidacja stanu i testy przerwań |
| DRIVE-033 architecture155: 2.04 | Rust hot path | 1.55 | REQUIRES_VERIFICATION / PROPOSED | Zdefiniować i zmierzyć rzeczywistą ścieżkę krytyczną |
| DRIVE-034 architecture155: 2.05 | Harness ewaluacji | 1.99 | REQUIRES_VERIFICATION / PROPOSED | Powtarzalny korpus, artefakty i niezależne wyniki |
| DRIVE-035 architecture155: 2.06 | Release guard | do ustalenia | DECISION_REQUIRED / UNMAPPED | Przypisać do istniejącej bramy wydania po ustaleniu wymagań; bez zmiany kanonicznego 2.06 |
| DRIVE-036 builder: AB-CLOUD-CODE | Agents Builder Chmurowy Code | 0.52 | NOT_IMPLEMENTED / EXACT | Browser Code IDE edits graph DSL, compiles/simulates typed agent, persists versioned project in tenant-scoped Cloud workspace; full build log |
| DRIVE-037 builder: AB-CLOUD-NOCODE | Agents Builder Chmurowy NoCode | 0.52 | NOT_IMPLEMENTED / EXACT | Browser NoCode canvas builds equivalent graph from nodes, round-trips graph JSON with compiled DSL for supported subset, and previews without source editing |
| DRIVE-038 builder: AB-WINDOWS-CODE | Agents Builder Desktopowy Windows Code | 0.53 | NOT_IMPLEMENTED / EXACT | Actual Windows runner builds, installs and launches signed/verified executable; Code IDE opens agent project, compiles valid Rust/DSL, persists/reopens offline |
| DRIVE-039 builder: AB-WINDOWS-NOCODE | Agents Builder Desktopowy Windows NoCode | 0.53 | NOT_IMPLEMENTED / EXACT | Actual Windows GUI smoke: NoCode drag/drop creates graph, save/reopen offline, convert supported graph to Code, publish artifact only with explicit approval |
| DRIVE-040 builder: AB-MACOS-CODE | Agents Builder Desktopowy macOS Code | 0.53 | NOT_IMPLEMENTED / EXACT | Actual macOS runner builds/boots executable, validates bundle codesign entitlement metadata, edits and compiles Agent Builder Code project offline |
| DRIVE-041 builder: AB-MACOS-NOCODE | Agents Builder Desktopowy macOS NoCode | 0.53 | NOT_IMPLEMENTED / EXACT | Actual macOS GUI smoke saves/loads NoCode agent, connects nodes, previews, converts and checks typed IR roundtrip, controls permissions |
| DRIVE-042 builder: AB-ANDROID-CODE | Agents Builder Android Code | 0.54 | NOT_IMPLEMENTED / EXACT | Actual Android ARM64 emulator/device installs APK, creates/edits Code agent in bounded DSL, saves locally offline and resumes after app restart |
| DRIVE-043 builder: AB-ANDROID-NOCODE | Agents Builder Android NoCode | 0.54 | NOT_IMPLEMENTED / EXACT | Android instrumentation creates visual NoCode graph, validates typed edges and safe preview, exports/imports offline, survives process death |
| DRIVE-044 builder: AB-IOS-CODE | Agents Builder iOS Code | 0.54 | NOT_IMPLEMENTED / EXACT | Actual Xcode iOS simulator/device run creates/edits Code agent under app Sandbox, validates typed agent DSL, restores local project after restart |
| DRIVE-045 builder: AB-IOS-NOCODE | Agents Builder iOS NoCode | 0.54 | NOT_IMPLEMENTED / EXACT | Actual iOS simulator UI builds multi-node NoCode workflow, round-trips supported IR with Code view, persists offline and respects app life-cycle |

Ścieżki dowodów i źródeł są zapisane w `drive_requirements_reconciliation.json`. Wyniki CI SASOS, SHM i Firecracker są opisane w `../DRIVE_044_049_IMPLEMENTATION_MAP.md`. Obecność demonstracyjnego kodu w PDF nie jest dowodem implementacji w repozytorium.
