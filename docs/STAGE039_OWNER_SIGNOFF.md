# Etap 0.39 — podpis właściciela

Właściciel projektu 2026-10-25 zrezygnował z niezależnego audytu. Bramka
`scripts/stage039_signed_acceptance.py` przyjmuje teraz wyłącznie schemat v2:
`audit_waiver=OWNER_WAIVES_INDEPENDENT_AUDIT` i jeden podpis Ed25519
właściciela. Schemat v1 z dwoma podpisami jest odrzucany.

Podpis obejmuje kanoniczny manifest JSON z commitem `main`, decyzją
`operator_decision=ACCEPT` i SHA-256 siedmiu osobnych plików dowodowych:

1. `two_jailed_tenant_guest_kernel_ebpf`
2. `independent_kernel_kvm_hosts`
3. `two_kernel_cold_restart`
4. `kernel_tls13_mtls_partition_fencing`
5. `production_interhost_mtls_ha`
6. `durable_host_partition_soak`
7. `operator_formal_approval`

Przykłady CI z efemerycznych hostów nie spełniają punktów 5–6.
Weryfikator sprawdza podpis i bajty załączników; sam wynik
`SIGNED_DOSSIER_INTEGRITY_VERIFIED_NOT_DEPLOYED` nie oznacza wdrożenia ani
nie ustawia `stage_039_complete=true`.

Po dostarczeniu rzeczywistych raportów z dwóch trwałych hostów operator
przygotowuje prywatny katalog 0700 z siedmioma **niepustymi, odrębnymi**
plikami dowodowymi. Plik `operator_formal_approval` zawiera świadomą
decyzję właściciela, oceniany commit i identyfikatory pozostałych dowodów.
Nie tworzyć tego pliku ani podpisu przed sprawdzeniem brakującego failover.
Klucza prywatnego nie należy umieszczać w repozytorium.

Właściciel generuje własny klucz tylko na swojej zaufanej maszynie. Jeżeli
ma już klucz Ed25519, pomija dwa pierwsze polecenia i eksportuje jego
klucz publiczny w formacie PEM. Ścieżki `/PRIVATE` oznaczają prywatny
katalog o prawach 0700, a pliki z kluczem prywatnym muszą mieć prawa 0600.

```bash
umask 077
openssl genpkey -algorithm Ed25519 -out /PRIVATE/operator.key
openssl pkey -in /PRIVATE/operator.key -pubout -out /PRIVATE/operator.pub
sha256sum /PRIVATE/operator.pub
```

Po wyliczeniu SHA-256 każdego z siedmiu plików operator tworzy
`/PRIVATE/dossier.json` z dokładnie tymi polami (bez dodatkowych pól):

```json
{
  "schema_version": 2,
  "stage": "0.39",
  "branch": "main",
  "git_commit": "PEŁNE_40_ZNAKÓW_SHA_COMMITU",
  "audit_waiver": "OWNER_WAIVES_INDEPENDENT_AUDIT",
  "operator_decision": "ACCEPT",
  "evidence": {
    "two_jailed_tenant_guest_kernel_ebpf": {"artifact_file": "guest-ebpf.json", "sha256": "SHA256_PLIKU"},
    "independent_kernel_kvm_hosts": {"artifact_file": "hosts.json", "sha256": "SHA256_PLIKU"},
    "two_kernel_cold_restart": {"artifact_file": "cold-restart.json", "sha256": "SHA256_PLIKU"},
    "kernel_tls13_mtls_partition_fencing": {"artifact_file": "partition.json", "sha256": "SHA256_PLIKU"},
    "production_interhost_mtls_ha": {"artifact_file": "failover.json", "sha256": "SHA256_PLIKU"},
    "durable_host_partition_soak": {"artifact_file": "soak.json", "sha256": "SHA256_PLIKU"},
    "operator_formal_approval": {"artifact_file": "approval.json", "sha256": "SHA256_PLIKU"}
  }
}
```

Wartości `SHA256_PLIKU` zastępuje się 64 znakami hex z `sha256sum` dla
faktycznych plików w `/PRIVATE/evidence/`; commit pochodzi z
`git rev-parse HEAD` po końcowym uruchomieniu testów. Następnie operator
kanonizuje manifest v2 przez `canonical_manifest` i podpisuje **dokładnie
te bajty**. Klucz publiczny i jego SHA-256 przekazuje osobnym kanałem.

```bash
PYTHONPATH=scripts python3 - /PRIVATE/dossier.json /PRIVATE/dossier.canonical.json <<'PY'
import json, sys
from pathlib import Path
from stage039_signed_acceptance import canonical_manifest
data = json.loads(Path(sys.argv[1]).read_bytes())
Path(sys.argv[2]).write_bytes(canonical_manifest(data))
PY
openssl pkeyutl -sign -inkey /PRIVATE/operator.key -rawin \
  -in /PRIVATE/dossier.canonical.json -out /PRIVATE/operator.sig
sha256sum /PRIVATE/operator.pub
```

Weryfikacja:

```bash
python3 scripts/stage039_signed_acceptance.py \
  --manifest /PRIVATE/dossier.json \
  --evidence-root /PRIVATE/evidence \
  --operator-pub /PRIVATE/operator.pub \
  --operator-pin SHA256_KLUCZA_PUBLICZNEGO \
  --operator-signature /PRIVATE/operator.sig \
  --report /PRIVATE/verified.json
```

Do formalnego zamknięcia potrzebny jest jeszcze semantyczny odbiór raportów
z dwóch trwałych hostów, 100 cykli na każdym i testu partycji między nimi.
Podpis właściciela następuje po tych próbach.
