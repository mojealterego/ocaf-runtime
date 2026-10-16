# PDF23 — audyt semantyczny (1–17 stron)

Źródło: `Architektura OmniContext Agent Framework (OCAF) w ..(3).PDF`. Nowość: **OmniStudio / OmniDoc**, strony 15–17. Status: **PLANNED_OR_UNVERIFIED**.

## Mapowanie wymagań
OmniStudio: Rezyser_Wizualny, graf generacji obrazu/wideo/audio, synchronizacja ścieżek i kontrola licencji oraz zgód; OmniDoc: Architekt_Typst, wersjonowane szablony, rzeczywista kompilacja Typst do PDF, walidacja plików, provenance i testy regresji. Kod źródłowy zwraca stałe nazwy MP4/WAV i ścieżkę PDF; brak wykonania.

Etapy istniejące: 0.43, 0.46, 0.47, 0.49, 0.50, 0.55, 0.63, 0.67, 0.98. Zachować 215 etapów głównych, 64 podetapy i 10 wariantów Agents Builder. Nowe roje nie są produktami Agents Builder. Pozostałe strony: wymagania wcześniej rejestrowane (Zenoh, CRDT, MCP, Firecracker, OAuth, L402, OmniDev, OmniUX, OmniE2E, OmniForge, OmniWeb); bibliografia i twierdzenia marketingowe nie są dowodami wdrożenia.

## Braki i kryteria odbioru
- **PDF23-001** (0.67): OmniStudio real media workflow graph and render. PDF23 pp.15-17. Wymagane: Run versioned DAG with real media backends; deterministic retry, resource budgets and output artifact validation.
- **PDF23-002** (0.63): OmniStudio audio-video synchronization and rights. PDF23 pp.15-17. Wymagane: Real audio alignment tests, explicit consent, provenance and license policy enforcement.
- **PDF23-003** (0.67): OmniDoc Typst compile and artifact verification. PDF23 pp.15-17. Wymagane: Compile actual Typst document, inspect produced PDF and test failure/timeout paths.
- **PDF23-004** (0.50): OmniStudio OmniDoc typed handoff and isolation. PDF23 pp.15-17. Wymagane: Typed schema, tenant isolation, scoped tool permissions and end-to-end traceability.

Nie zaliczać przykładowych funkcji, stałych ścieżek, komunikatów `println!`, prognoz Gartnera/Juniper ani obietnic „100%” jako testów, integracji lub certyfikacji. Dla OmniPhys wymagane niezależne testy bezpieczeństwa fizycznego; nie wykonywać poleceń na rzeczywistych urządzeniach bez uprawnień.
