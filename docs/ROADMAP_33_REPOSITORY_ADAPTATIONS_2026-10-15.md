# OCAF — rozszerzenie pełnej Roadmapy o 33 referencyjne repozytoria (2026-10-15)

**Status: projekt wymagań i backlog, nie zaświadczenie implementacji.** Repo: `mojealterego/ocaf-runtime`, wyłącznie `main`.

## Pochodzenie materiału

Na prośbę operatora uwzględniono dwa dostarczone opracowania PDF:
- `Dokument bez nazwy(1).PDF` — **„Audyt Inżynieryjny i Dekonstrukcja 24 Repozytoriów”** (13 stron, sześć grup, 24 nazwy).
- `Dokument bez nazwy.PDF` — **„Audyt Inżynieryjny i Dekonstrukcja 9 Repozytoriów”** (14 stron, cztery grupy, dziewięć nazw).

Przeniesiono zakres funkcjonalny do istniejących etapów OCAF, zamiast tworzyć 33
oderwane silniki lub kopiować bez testów demonstracyjne `main.rs` z PDF. Nazwy
repozytoriów i wydajność w PDF traktowane są jako **hipotezy i inspiracje
architektoniczne**, nie wyniki niezależnego audytu kodu źródłowego. Repozytoria
o podobnych nazwach wymagają osobnej weryfikacji URL/licencji/commitu upstream.
Nie dodano automatycznie obcych zależności ani kodu źródłowego do `Cargo.lock`.

## 24 repozytoria — macierz adaptacji (PDF 13 stron)

| Grupa | Źródła w dokumencie | Docelowy moduł OCAF i etap integracji | Kryteria wejścia i negatywne testy |
|---|---|---|---|
| P24-01: orkiestracja agentów | `autogen`, `AutoAgent`, `babyagi`, `bisheng`, `Awesome-Serverless-Visual-Workflow-Orchestration` (5) | `OmniAutoGen-ActorBus`, `DampedBabyAgiPlanner`; 0.46, 0.50, 0.70, 0.71 | Typowane wiadomości, limity liczby zadań/głębokości/fan-out/tokenów, idempotencja, przerwanie drzew zapętlonych, brak eskalacji uprawnień. |
| P24-02: builder i darmowe API | `bolt.diy`, `base44-platform-starter`, `awesome-free-llm-apis` (3) | `OmniBolt-Workstation`, `OmniFreeApi-Rotator`; 0.50–0.55, 0.56, 0.69 | Tree-sitter AST/CST diffs, CAS/optimistic locking, atomic rollback; twarde RPM/TPM i per-tenant billing caps, jawna zgoda przed fallbackiem na płatne L402. |
| P24-03: Skills/MCP | `book-to-skill`, `awesome-agent-skills`, `awesome-mcp-servers`, `Azure-mcp`, `awesome-selfhosted` (5) | `OmniBookToSkill-Compiler`, `OmniAzureMcp-Bridge`; 0.43–0.44, 0.48, 0.62 | Cytowana struktura źródeł, ekstrakcja CoALA z warunkami brzegowymi, kontrakt JSON Schema, podpisane rejestry capabilities, przeciwdziałanie prompt injection i podszywaniu się pod dostawcę. |
| P24-04: obrona i audyt | `AWD-Predator-Framework`, `autopentest-ai`, `BlackOps-Armarda`, `Awesome-AI-Hacking-Agents`, `Awesome-Hacking` (5) | `OmniPredator-AwdDefenseGrid`, `OmniSec`; 0.39.7, 0.47, 0.63, 0.70 | Telemetria Aya/eBPF tylko przy rzeczywistym kernel attach; scope/consent przed jakąkolwiek diagnostyką; signed policy, canary virtual-patch, rollback; brak nieautoryzowanego skanowania. |
| P24-05: głos, wideo, QASM | `awesome-phone-call-agents`, `awesome-voip`, `Awesome-LLMs-meet-Multimodal-Generation`, `awesome-seedance-prompts`, `Awesome-Quantum-Computing-Platform` (5) | `OmniVoip-RtpEngine`, `OmniSeedance-Director`, `OmniQuantum-Qasm`; 0.61, 0.75, 0.76 | RTP jitter/packet loss benchmarks, consent + PII, media rights, finite camera graphs, OpenQASM 3 parser w odrzucaniem błędnych indeksów i niezaufanych bramek. |
| P24-06: prywatność WWW | `brave-browser` (1) | `OmniBrave-ContextShield`; 0.48, 0.60, 0.83 | Parser HTML DOM, canonicalizacja URL, polityka redirect/SSRF i zasobów 3rd party, oddzielenie treści WWW od instrukcji narzędzi, payloady obfuskowane i fallback bez wstrzyknięcia. |

**Razem: 24 nazwy (5+3+5+5+5+1).**

## 9 repozytoriów — macierz adaptacji (PDF 14 stron)

| Grupa | Źródła w dokumencie | Docelowy moduł OCAF i etap | Testy akceptacji / ochrona |
|---|---|---|---|
| P09-01: analiza i kontrakt | `aider`, `ArchGen` (2) | `OmniAider-RepoMap`, `OmniArchGen-Contract`; 0.50, 0.55–0.56 | PageRank symboli z normalizacją/obsługą dangling nodes, atomowe patche i CAS, cykle DAG, blokada zabronionych krawędzi między View/Core/Data/Infrastructure, deterministyczne snapshoty. |
| P09-02: audyt zakresu | `artemis`, `AI_Offensive_MCP_Using_KaliLinux`, `AllHackingTools` (3) | `OmniArtemis-AuditMCP`; 0.43, 0.63 | Wyłącznie autoryzowane domeny/CIDR, kontrola DNS rebinding i redirectów przed połączeniem, ograniczone rate/timeouts, pasywne kontrole TLS/CSP/HSTS/CORS/DMARC; brak surowego shell/command injection. |
| P09-03: studio web/mobile | `appsmith`, `Android-AI-Studio` (2) | `OmniAppStudio-Engine`; 0.50–0.55, 0.58 | Typed widget bindings, Zenoh topic ACL, non-authoritative CRDT merge, offline Android Compose, explicit sync policy i E2E permissions. |
| P09-04: vertical matching | `AI-Rental-Hunter`, `AI-Resume-Optimizer` (2) | `OmniVertical-Matcher`; 0.67, 0.69, 0.71 | Deduplikacja i fair housing/privacy, odporna mediana/MAD anomalii z wyjątkiem zero-variance, ATS-only claims ze zweryfikowanymi referencjami do faktów, brak fikcyjnych doświadczeń. |

**Razem: 9 nazw (2+3+2+2). Łącznie: 33 nazwy.**

## Dokładny harmonogram i priorytety

1. **P0: 0.39.4–0.39.10** — zakończyć testy realnego KVM i izolację Firecracker + Aya na autoryzowanym hoście; nie mieszać tego z parserami dokumentów, kodem LLM ani backendem UI.
2. **P1: 0.43, 0.50, 0.55, 0.56, 0.63** — przygotować wspólne kontrakty typed DAG, manifest MCP i scope audit, repo map/AST CAS oraz bezpieczny workflow execution. W nich realizować P24-01, P24-03, P24-04, P09-01/02.
3. **P1/P2: 0.46, 0.48, 0.60, 0.67, 0.69–0.71** — SHM i bounded task recursion, OmniBrave i niskokosztowe routing/fallback, OmniVertical; mikrobenchmarki dopiero po działających adapterach.
4. **P2: 0.52–0.54, 0.58, 0.61–0.62, 0.75–0.76** — web/Android studio, procesory multimediów/VoIP i walidator QASM w podpiętych sandboxach.

## Rzeczywisty zakres implementacji vs przykładowe kody PDF

Szkice w PDF są **demonstracjami**, a nie gotową implementacją pod OCAF.
Przed włączeniem wymagają m.in.:
- `DampedBabyAgiPlanner`: samo `b_d = b_0 e^{-γd}` obniża priorytet, **nie
  gwarantuje terminacji**. Twardo wymuszać głębokość, maks. liczbę
  zadań, fan-out, budżet i deadline w ledgerze.
- `OmniBrave-ContextShield`: tekstowe `replace()` nie neutralizuje
  wielu wariantów HTML/DOM ani prompt injection; sandbox i typowana
  granica treść/instrukcja są obowiązkowe.
- `OmniPredator-AwdDefenseGrid`: dopisanie nazwy wzorca do pamięci
  **nie jest** attach programu eBPF ani hot-patching WAF; testować
  faktyczny BPF link ID i egzekwowanie polityki.
- `OmniArtemis-AuditMCP`: porównanie surowego napisu nazwy hosta z
  listą domen **nie** zabezpiecza połączeń po przekierowaniu/DNS
  rebinding. Zatwierdzone rozstrzygnięcie IP i egress policy wymagane.
- `OmniFreeApi-Rotator`: tablica demonstracyjnych RPM i latencji
  **nie** stanowi rzeczywistych limitów z API; paid fallback bez
  wyraźnego opt-in jest niedozwolony.
- `OmniVertical-Matcher`: brak dowodu pochodzenia elementów CV
  nie może zostać zastąpiony heurystyką dopasowania słów.
- `OmniQuantum-Qasm`: generowanie napisu QASM nie jest symulatorem
  kwantowym ani dowodem zgodności z gramatyką OpenQASM 3.

## Wymagany stan dla każdego pakietu

Każda implementacja ma: `upstream URL + pinned commit + license check`,
threat model, dopuszczalne capabilities, testy jednostkowe i integracyjne
na realnym backendzie, testy negatywne, benchmark z powtarzalnością,
profil targetów Linux/Android, reversible deployment, log audytowy oraz
właściciela akceptacji. **Kolejność nie jest równoznaczna z
produkcją** — odblokowanie administracyjne operatora jest osobną decyzją.
