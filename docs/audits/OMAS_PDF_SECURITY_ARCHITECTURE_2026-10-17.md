# Audyt źródła: „Audyt Kodu i Projekt OMAS” (29 stron, 2026-10-17)

**Klasyfikacja:** analiza wymagań i ocena dostarczonego w PDF przykładowego kodu; NIE jest to dowód wdrożenia, kompilacji ani testu OCAF. Źródło: przesłany PDF, strony 1–29.

## Mapa wymagań do istniejącej roadmapy

| PDF | Wymaganie / problem | Przypisanie | Kryterium odbioru / dowód | Status |
|---|---|---|---|---|
| 1–2 | Fail-closed uwierzytelnianie; nagłówek `x-mcp-issuer` nie jest uwierzytelnieniem | auth / tool gateway; istniejące etapy bezpieczeństwa | weryfikacja tożsamości kryptograficznej, scope, tenant i negatywne testy bez tokenu | GAP — wymaga kontroli kodu |
| 2 | AST/regex `.contains()` nie jest dowodem bezpieczeństwa; formal verifier nie może fail-open | security / weryfikacja kodu | testy bypass, timeout prover -> deny; podpisany wynik rzeczywistego prover | GAP |
| 2–3 | TOCTOU workspace, `gradlew`, argument injection `g++`, Docker socket | sandbox, 0.39 i kompilatory Builder | per-job workspace, path confinement, argv validation, sandbox bez host root, testy równoległe | GAP |
| 3–4 | nieograniczone mapy pamięci, entropia SHA-256 jako pseudometryka | memory / observability | limity bajtów i wpisów, TTL, testy obciążenia; udokumentowana definicja Shannon H(X) | GAP |
| 3 | Zenoh key-expression injection | transport 0.46, tool gateway | walidacja segmentów namespace, wildcard deny, testy tenant-crossing | GAP |
| 5–7 | Zenoh ZBytes/SHM, typed rkyv, benchmark zero-copy | transport 0.46 | wersjonowane schema/ABI, walidacja nieufnych ramek, E2E i pomiary p50/p99 na sprzęcie | PLANNED/UNVERIFIED |
| 7, 17–19 | Wasmtime fuel, epoch, limity zasobów i brak importów | runtime / 0.39 | bounded execution, timeout/cancel, memory/stack caps, signed guest WASI E2E | GAP |
| 8, 14–17 | Ed25519, nonce, timestamp, namespace, quotas | auth / 0.39 / 0.46 | pełna kanoniczna koperta podpisu, trwały replay cache, key rotation, fail-closed | GAP |
| 8, 12–14 | bounded R3Mem i Shannon entropy | memory / metrics | limit bajtowy, FIFO/LRU spójność, deterministyczne testy i telemetria | GAP |
| 20–27 | pięć operacji OMAS: ExecuteSandboxedWasm, SimulateAndMemorizeTrajectory, DispatchSwarmTask, BindDynamicCapability, InspectKernelState | tool gateway / swarm / registry / 0.46 | typed contracts, RBAC per opcode, limity odpowiedzi, idempotencja, testy E2E | GAP |

## Krytyczne błędy w kodzie OMAS zaprezentowanym w PDF

1. **Replay-cache przed weryfikacją podpisu** (s. 15–16): napastnik może zatruwać nonce; walidacja podpisu powinna poprzedzać atomowe, trwałe zarejestrowanie nonce.
2. **Podpis nie obejmuje `op_code`, `fuel_quota` ani `invariant_fuel_limit`** (s. 16–17). Podpisane żądanie można przekształcić w inną operację lub zmienić limity bez unieważnienia podpisu. Wymagany canonical encoding z domain separation, wersją, tenantem, opcode, targetem, limitem, timestamp, nonce i hashem payload.
3. **Prefix confusion** (s. 16): `target_key.starts_with(allowed_ns)` bez granic segmentów pozwala na niezamierzone prefiksy; należy autoryzować kanoniczne segmenty i tenant.
4. **Replay-cache jest wyłącznie w RAM i usuwa nonce przy limicie 8192** (s. 14–15); ponowne użycie starego nonce w nadal ważnym oknie może zostać zaakceptowane. Wymagane atomowe TTL/replay-store na okres ważności, recovery i testy restartu.
5. **Brak limitu bajtowego dla payload i agregacji odpowiedzi roju** (s. 20–23). Limit liczby rekordów pamięci nie ogranicza zużycia RAM, `aggregated.extend_from_slice` nie ma limitu; potrzebne max-frame/max-result/max-memory.
6. **Dynamic capability bind** (s. 23–24) opiera się na samym prefiksie nazwy; brak odrębnego scope operacji i polityki routingu.
7. **Samotest nie dowodzi wdrożenia** (s. 27–28): loguje odpowiedź bez asercji statusu i wyniku, używa lokalnego WAT, nie Firecracker ani podpisanego WASI w gościu.
8. **Twierdzenia o wydajności 7–10 µs, 12–40x tokenów i „zero alokacji”** (s. 5–9, 28–29) są hipotezami/porównaniami ze źródeł wtórnych, nie benchmarkiem OCAF. Nie przyjmować jako SLA.
9. **Oznaczenie „gotowe produkcyjnie”** (s. 9) nie jest uzasadnione testami, wdrożeniem, niezależnym audytem ani walidacją wszystkich granic bezpieczeństwa.

## Decyzja architektoniczna

Nie zastępować istniejącego runtime OCAF osobnym jądrem OMAS. Rozważyć OMAS jako **wersjonowany natywny protokół wewnętrzny i zestaw kontraktów** nad już istniejącymi `transport`, `auth`, `executor`, `registry` i `metrics`; adapter MCP/OpenAPI może pozostać na granicy integracyjnej. Brak uzasadnienia dla usunięcia HTTP/MCP wszędzie. Nie deklarować wydajności bez benchmarku, formalnej weryfikacji bez dowodu ani produkcyjności na podstawie kodu w PDF.

**Status audytu:** dokument przeanalizowany semantycznie; przypisania do modułów wymagają dalszego sprawdzenia kodu/CI. Żaden wymieniony GAP nie został automatycznie uznany za wdrożony.
