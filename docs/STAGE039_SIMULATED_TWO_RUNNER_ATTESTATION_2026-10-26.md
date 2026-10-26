# 0.39 — podpisany raport próby zastępczej, 2026-10-26

Na polecenie właściciela wykonano próbę zastępczą na runnerach GitHub Actions.
Podpis jest **jawnym podpisem delegowanego świadka technicznego**, wykonanym
nowym kluczem Ed25519 w tej sesji. Nie jest osobistym podpisem właściciela ani
formalnym odbiorem etapu 0.39. Klucz prywatny nie został dodany do repozytorium
i został usunięty po podpisaniu.

| Próba | Wynik | Granica dowodu |
| --- | --- | --- |
| [Dwa osobne runnery KVM](https://github.com/mojealterego/ocaf-runtime/actions/runs/40280840038) | Sukces: 100 cykli boot/SIGKILL na każdym runnerze; porównanie dwóch boot ID. | Runnery są efemeryczne; nie dowodzi trwałości hostów ani bezpośredniej komunikacji między nimi. |
| [Odtworzenie checkpointu](https://github.com/mojealterego/ocaf-runtime/actions/runs/40280840120) | Sukces: drugi runner uruchomił nową jailed VM i odtworzył nonce z artefaktu pierwszego. | Transfer przez artefakt GitHub; nie jest to żywy failover ani replikacja brokera. |
| [Partycja mTLS](https://github.com/mojealterego/ocaf-runtime/actions/runs/40280839982) | Sukces: TLS 1.3, odmowa w trakcie partycji, fencing i wznowienie. | Dwie przestrzenie sieciowe na jednym jądrze; nie jest to partycja między fizycznymi hostami. |

Wszystkie przebiegi użyły commita `5f3bc65936aa913e834045cfa36b56589d86cda5`.
[Raport JSON](evidence/STAGE039_SIMULATED_TWO_RUNNER_ATTESTATION_2026-10-26.json)
zawiera nazwy, identyfikatory, rozmiary i digesty archiwów zgłoszone przez
GitHub API. Pobranie bajtów archiwów z tego środowiska zakończyło się błędem
Azure Blob 403, dlatego podpis obejmuje **metadane i wyniki bramek CI**, a nie
niezależnie odczytaną zawartość archiwów.

Podpisano dokładne bajty pliku JSON. Pliki:
[klucz publiczny](evidence/STAGE039_SIMULATED_TWO_RUNNER_ATTESTATION_2026-10-26.pub.pem)
i [podpis Base64](evidence/STAGE039_SIMULATED_TWO_RUNNER_ATTESTATION_2026-10-26.sig.b64).
SHA-256 klucza publicznego PEM:
`c88b33d1cbeac13471effbe86d1aeeaf2d8821023062ab9aae72e324948af8b2`.
Weryfikacja:

```bash
base64 -d docs/evidence/STAGE039_SIMULATED_TWO_RUNNER_ATTESTATION_2026-10-26.sig.b64 > /tmp/ocaf039-simulation.sig
openssl pkeyutl -verify -pubin \
  -inkey docs/evidence/STAGE039_SIMULATED_TWO_RUNNER_ATTESTATION_2026-10-26.pub.pem \
  -rawin -in docs/evidence/STAGE039_SIMULATED_TWO_RUNNER_ATTESTATION_2026-10-26.json \
  -sigfile /tmp/ocaf039-simulation.sig
```

Klucz publiczny przechowywany obok raportu potwierdza spójność podpisu, lecz
sam nie dowodzi tożsamości właściciela. Raport jawnie zapisuje
`owner_personal_signature=false`, `two_persistent_hosts_tested=false`,
`production_interhost_ha_tested=false` i `stage_039_complete=false`.
Formalna bramka [podpisu właściciela](STAGE039_OWNER_SIGNOFF.md) pozostaje
oddzielna. Wystawienie niniejszego podpisu nie zmienia statusu produkcyjnego.
