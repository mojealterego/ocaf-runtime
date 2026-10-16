# PDF24 — audyt semantyczny (1–21 stron)

Źródło: `Architektura OmniContext Agent Framework (OCAF) w ..(4).PDF`. Nowość: **OmniGov / OmniFin / OmniPhys**, strony 15–18. Status: **PLANNED_OR_UNVERIFIED**.

## Mapowanie wymagań
OmniGov: Straznik_Budzetu i Audytor_Gov — egzekwowane limity tokenów, L402, uprawnień i zatrzymanie kosztownych pętli; OmniFin: metering, FinOps, koszt CPU, tokenów i mikropłatności, dashboard ROI z metodologią; OmniPhys: KoordynatorIoT_Physical, sterowanie urządzeniami brzegowymi przez Zenoh z autoryzacją, interlockami, fail-safe i testami HIL. PDF zawiera wyłącznie logowanie i statyczny raport audytu; brak prawdziwego odcięcia dostępu, księgowania i urządzeń.

Etapy istniejące: 0.38, 0.39, 0.43, 0.49, 0.55, 0.63, 0.67, 0.98. Zachować 215 etapów głównych, 64 podetapy i 10 wariantów Agents Builder. Nowe roje nie są produktami Agents Builder. Pozostałe strony: wymagania wcześniej rejestrowane (Zenoh, CRDT, MCP, Firecracker, OAuth, L402, OmniDev, OmniUX, OmniE2E, OmniForge, OmniWeb); bibliografia i twierdzenia marketingowe nie są dowodami wdrożenia.

## Braki i kryteria odbioru
- **PDF24-001** (0.49): OmniGov enforced pre-authorization budget guard. PDF24 pp.16-18. Wymagane: Atomic pre-spend reservation, wallet revocation, tenant quotas, race and replay tests.
- **PDF24-002** (0.63): OmniGov independent policy audit and tamper-evident records. PDF24 pp.16-18. Wymagane: Fail-closed policy engine, immutable signed event evidence, separation of duties.
- **PDF24-003** (0.67): OmniFin metering and ROI accounting. PDF24 pp.16-17. Wymagane: Reconcile LLM tokens, Lightning settlement and compute cost to invoices with audit-ready evidence.
- **PDF24-004** (0.39): OmniPhys safe edge control and emergency stop. PDF24 pp.17-18. Wymagane: Device identity, scoped commands, watchdog, safety interlocks, offline safe state and hardware-in-loop tests.
- **PDF24-005** (0.98): OmniGov cost exhaustion adversarial E2E. PDF24 pp.16-18. Wymagane: Prove wallet cutoff, bounded spend and recovery under concurrent attack and network partition.

Nie zaliczać przykładowych funkcji, stałych ścieżek, komunikatów `println!`, prognoz Gartnera/Juniper ani obietnic „100%” jako testów, integracji lub certyfikacji. Dla OmniPhys wymagane niezależne testy bezpieczeństwa fizycznego; nie wykonywać poleceń na rzeczywistych urządzeniach bez uprawnień.
