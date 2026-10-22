# OMAs — osobny serwer agentów

OMAs jest odrębnym procesem odpowiedzialnym za rejestrację agentów, tenant
boundary, replay protection, lifecycle i health. Nie wykonuje narzędzi; to
należy do OMAt. Zenoh jest wyłącznie transportem.

## Uruchomienie lokalnego kontraktu

```text
cargo run --no-default-features --bin ocaf-omas-server -- --health
```

Proces bez `--health` czyta z `stdin` pojedyncze obiekty JSON
`AgentRegistrationRequest` i emituje decyzje JSON. Jest to lokalny adapter
kontraktu, a nie dowód produkcyjnego endpointu sieciowego.

## Bramka bezpieczeństwa

- `ostc_enabled` musi być `false`;
- `source_access` musi być `disabled`;
- `ACTIVE` i fałszywe `readiness` są odrzucane;
- manifest musi mieć digest `sha256:*`;
- tenant i agent są walidowane, a `request_id` jest jednokrotny;
- rejestracja kończy się stanem `STAGED`;
- OMAs nie nadaje dostępu do plików źródłowych i nie uruchamia narzędzi.

## Stan wdrożenia

Kod kontraktu i testy znajdują się w `src/omas_server.rs`. Status komponentu
pozostaje `PARTIAL`: brakuje osobnego procesu sieciowego z TLS/mTLS, trwałego
rejestru, testu wielodzierżawnego E2E i niezależnego artefaktu deploymentu.
Nie wolno podnosić statusu do `ACTIVE` samą deklaracją gotowości.
