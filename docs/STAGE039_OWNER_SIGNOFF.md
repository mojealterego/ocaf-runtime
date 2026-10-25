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
przygotowuje prywatny katalog 0700 z siedmioma plikami dowodowymi.
Następnie tworzy manifest v2 przez `canonical_manifest` z
`scripts/stage039_signed_acceptance.py`, podpisuje dokładnie wyjściowe
bajty Ed25519, a klucz publiczny i jego SHA-256 przekazuje osobnym kanałem.
Klucza prywatnego nie należy umieszczać w repozytorium.

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
