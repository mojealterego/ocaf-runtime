# OCAF SASOS — etap 334: rzeczywiste adaptery i infrastruktura

**Status: IMPLEMENTED_PARTIAL / EXTERNAL_E2E_NOT_VERIFIED / PRODUCTION_NO_GO.**
Dotyczy istniejącego runtime Rust i katalogu 555 deklaracji, nie drugiego rdzenia.

## OSTc — odłożone, nie usunięte

Na decyzję operatora atestacja OSTc jest ostatnią bramką. Do tego
czasu nie odblokowywać `OmatRegistry::execute`, native, WASI,
Firecracker ani żadnej ścieżki z dostępem do źródeł lub powłoki.
Zachowujemy `SourceAccessDenied` dla dotychczasowego OMAt.
Niezależna ścieżka **wyłącznie operator-bound HTTPS** korzysta
z istniejącego `VaultSecretBroker`, `AuthorizedGrant` i
`BrokerRoute` (TLS, publiczny przypięty IPv4, port 443,
bez przekierowań i URL z argumentów modelu). Nie przyznaje
uprawnień jądra ani arbitralnego wykonywania kodu.

## Rzeczywisty adapter HTTPS — bez fałszywej aktywacji 555

- `src/sasos_service_adapters.rs` — `SasosCatalog::from_json`
  weryfikuje 555 rekordów i odrzuca fałszywe `executable=true`.
- `SasosAdapterBindings::from_private_file` ładuje **wyłącznie
  z prywatnego pliku operatora** jawne powiązania
  `sasos_tool_id`, `route_id`, `broker_tool_id`,
  `max_body_bytes`. Zasada deny-by-default; brak automatycznego
  mapowania 555 wpisów na endpointy.
- `prepare_call` wymaga dokładnie zadeklarowanych pól i limitu
  bajtów; dodatkowe pola, null, brak bindingu i Ring0 są odrzucane.
- `execute` wywołuje prawdziwy `VaultSecretBroker.execute`:
  grant OAuth, izolacja tenant/user, odszyfrowanie tokenu przez Vault
  Transit i HTTPS POST na przypięty operatorowi endpoint.
  Wynik inny niż 2xx jest błędem. Nie wystawiamy tej funkcji
  jako otwartego proxy ani nie traktujemy jej jako testu 555 backendów.
- `tests/sasos_service_adapters.rs` — testy kontraktowe i negatywne.
  Zewnętrzny E2E nie może być zaliczony bez rzeczywistego dostawcy,
  operatora i dowodu odpowiedzi usługi.

Przykład **formatu** prywatnego pliku bindingów (nie stan aktywacji):

```json
[
  {
    "sasos_tool_id": "SASOS_TOOL_004_BEFORESUNSET_MCP_COORDINATOR",
    "route_id": "operator-approved-route",
    "broker_tool_id": "operator-approved-tool",
    "max_body_bytes": 4096
  }
]
```

Binding nie wystarczy: `VaultSecretBroker` musi posiadać dokładnie
zgodny `BrokerRoute` i osobny, rzeczywiście zaprowizjonowany sekret
dla tenant/user/route. Wpisy Ring0 i Ring1 są blokowane bez OSTc; tylko Ring2Worker może uzyskać jawny binding HTTPS.

## Infrastruktura IdP, Vault, KVM

1. Wdróż Keycloak (lub równoważny rzeczywisty IdP) z TLS 1.3,
   Authorization Code + PKCE S256, MFA/ACR, ekranem zgody,
   poprawnym JWKS i pinningiem issuer/kid. Szablon:
   `deploy/stage044/keycloak-realm.template.json`.
2. Wdróż Vault Transit z TLS 1.3, prywatnym tokenem serwisowym,
   kluczem derived/non-exportable, polityką
   `deploy/stage044/vault-policy.hcl`, rotacją i HA.
   Weryfikacja HSM wymaga osobnego sprzętowego artefaktu.
3. Na **operator-owned** hoście x86_64 z `/dev/kvm`, uruchom
   test rzeczywistego Firecracker guest/jailera z istniejącego
   `.github/workflows/stage039-real-firecracker-jailer.yml`.
   KVM ioctl nie jest bootem Firecracker i nie jest atestacją OSTc.
4. Skonfiguruj prywatne pliki `0600` i GitHub Actions vars
   `OCAF_E2E_ISSUER`, `OCAF_E2E_GATEWAY`,
   `OCAF_E2E_RESOURCE`, `OCAF_E2E_TENANT`,
   `OCAF_E2E_USER`, `OCAF_E2E_PROVIDER`,
   `OCAF_E2E_TOKEN_FILE`, `OCAF_VAULT_ADDR`,
   `OCAF_VAULT_KEY`, `OCAF_VAULT_TOKEN_FILE`.
   Tokenów nie umieszczać w GitHub vars ani w repo; wskazane
   pliki muszą istnieć na hostcie.
5. Nadaj operator-owned runnerowi etykietę `ocaf-stage334-e2e`
   i ręcznie uruchom
   `.github/workflows/sasos334-external-acceptance.yml`.
   Brak runnera, konfiguracji, IdP, Vault lub KVM oznacza FAIL
   albo kolejkę, nie automatyczny PASS.

## SHM — prawdziwy test jądra, nie potwierdzenie Zenoh

`scripts/sasos_shm_probe.py` tworzy realny Linux memfd,
przekazuje deskryptor osobnemu procesowi, porównuje SHA-256,
wymaga ACK i odrzuca modyfikację danych. Testuje wspólną
pamięć **na jednym hoście**. Nie testuje jeszcze przekazania
Zenoh `ZShm`/negocjacji transportu, nie dowodzi zero-copy
między hostami i nie aktywuje feature `shared-memory` w runtime.
**Zweryfikowane dodatkowo na Zenoh 1.10.1:** dwa odrębne procesy
Zenoh Python (Rust binding), 32768-bajtowy ZShm i test fallback na
zwykły ZBytes przy wyłączonym SHM po stronie odbiorcy.
[Run 40942073444](https://github.com/mojealterego/ocaf-runtime/actions/runs/40942073444)
PASS, [artefakt 12927641458](https://github.com/mojealterego/ocaf-runtime/actions/runs/40942073444/artifacts/12927641458).
To test biblioteki Zenoh na jednym hoście; połączenie tej ścieżki
z produkcyjnym OMAt Rust, mTLS, failover i wielohostową polityką
nadal jest otwarte. Zero-copy między hostami nie jest wymaganym ani
prawdziwym trybem SHM.

## Solver i modele pamięci

`verification/sasos334/*.smt2` i
`scripts/sasos_solver_evidence.py` wykonują rzeczywisty Z3:
3 modele oczekują `unsat`, negatywna kontrola oczekuje `sat`.
Raport zawiera wersję solvera i SHA-256 każdego modelu.
Jest to dowód dla **ograniczonych formuł**, nie dowód formalny
Rust, nie ImandraX ani dowód bezpieczeństwa całego runtime.

`scripts/sasos_memory_models.py` implementuje:
- `LosslessTenantArchive` — zlib, SHA-256, atomowe
  no-clobber, prywatne katalogi i kontrolę odtwarzania bajtów;
- `SurpriseGatedMemory` — bounded EMA z squared-L2 surprise,
  limitem pojemności, izolacją tenantów i checkpointem.

To **działające algorytmy referencyjne**, ale nie wytrenowany
Titans, NICE ani R3Mem; brak dowodów jakości, model weights,
pracy na rzeczywistych korpusach i regresji. Nie deklarować
`Titans/NICE VERIFIED`.

## CI i odbiór

`.github/workflows/sasos-activation-contract.yml`:
katalog, testy adaptera, solver Z3, Linux memfd i negatywne
testy OAuth/capability. Wynik wiązać z konkretnym commit SHA.
`.github/workflows/sasos334-external-acceptance.yml`:
ręczny, realny IdP/Vault/KVM ioctl na hoście operatora.
`stage039-real-firecracker-jailer.yml`: osobna bramka guest boot.
Żaden z tych wyników samodzielnie nie zamyka etapu 334.

## Zweryfikowane GitHub Actions (przypisane do źródła)

- [Etap 334 — PASS, run 40941557248](https://github.com/mojealterego/ocaf-runtime/actions/runs/40941557248),
  commit `261f8ed7a268eff4a771bce77b2ef6efbc7695bc`:
  walidacja 555 deklaracji, Z3, memfd, testy pamięci,
  `cargo fmt`, `cargo test` capability/adapter/OAuth i `cargo clippy`.
  Dowodzi laboratoryjnych kontraktów, nie uruchomienia usług.
- [Zenoh SHM — PASS, run 40942073444](https://github.com/mojealterego/ocaf-runtime/actions/runs/40942073444),
  commit `d52c9d9f18c4363eebedf266fdee0e079578ae2a`:
  dwuprocesowy SHM i kopia/fallback na jednym kernelu.
- **Niezweryfikowane:** self-hosted `sasos334-external-acceptance.yml`,
  poświadczenia realnego IdP/Vault, operator-owned Firecracker,
  komplet 555 providerów, pełny Titans/NICE, formalny dowód programu,
  produkcyjny OMAt SHM i operator acceptance. Brak PASS dla tych obszarów.
