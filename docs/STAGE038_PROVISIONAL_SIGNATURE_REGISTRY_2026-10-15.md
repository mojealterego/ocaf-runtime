# OCAF 0.38 — rejestr zgłoszonych sygnatariuszy i techniczne poświadczenie utworzenia kluczy

**Rodzaj dokumentu: PROVISIONING ONLY / NIE JEST FORMALNYM ODBIOREM ANI NIEZALEŻNYM AUDYTEM.**
Data wygenerowania: 2026-10-14T22:26:16Z. Repozytorium: `mojealterego/ocaf-runtime`, gałąź wyłącznie `main`.

Użytkownik zgłosił zgodę na przygotowanie podpisów kryptograficznych dla nazw:
**mojealteregopl**, **fotografandrzej333**, **zchiz444**.
Nie przeprowadzono weryfikacji, że rozmówca jest właścicielem każdej nazwy,
że trzy nazwy oznaczają trzy niezależne osoby ani że któraś z nich posiada
uprawnienia niezależnego audytora lub operatora.

Zgodnie ze zgłoszeniem utworzono **trzy odrębne pary kluczy Ed25519**,
z których wykonano i pomyślnie zweryfikowano **trzy rzeczywiste podpisy
kryptograficzne** pod wyłącznie technicznym zapisem zgłoszenia zgody.
Wszystkie klucze powstały w **jednym środowisku wykonawczym asystenta**
i z tego powodu **nie są niezależnymi podpisami właścicieli kont**.
Nie wolno wykorzystać tych podpisów jako `auditor.sig` lub `operator.sig`
w produkcyjnym `stage038_production_gate.py`.

| Wskazana nazwa | SHA-256 klucza publicznego DER | SHA-256 podpisu technicznego | Tożsamość i niezależność |
|---|---|---|---|
| `mojealteregopl` | `1f0bfe5bdb48f05602ef34097d1bdcdfd17c32d76f0daa1f41b27428a8b21109` | `783a9946acbcfd31aecd9ac46468915d656daae6798443e7be87480ceef092c0` | Niezweryfikowana |
| `fotografandrzej333` | `0cfbab09873db1bc52e6c3efaa869045f7e8863bddf2e5ba62740938c4df9571` | `591e48a5b883f8cd87c6f0152d6ed2b86eb4fc64404bb58aa662ca292f4ac8ca` | Niezweryfikowana |
| `zchiz444` | `ffc041e8a3c1d090021f4025952d98f7ac4df918f9c15595ec435c70d0c75d9d` | `9ee7eda3ffc37e67b951d012bba2a9843917ea79e54987bcf27c5e6a71815d27` | Niezweryfikowana |

## Publiczne klucze PEM i podpisy base64

### mojealteregopl
```pem
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAVIK+KlInmkKeJHuFTEpOq0nJGnT7YJx+PWfPuw11/lk=
-----END PUBLIC KEY-----
```
Podpis `base64`: `iE6tN+roI9/7x9vlNEvcdwQkrjSzOg+VyW9z2FSEkf7AIuedJ0EEEcIBJhOujrdF6b87e31/41SF2irEnwgyAg==`

### fotografandrzej333
```pem
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAgUyHmJC/SLnvByRNhnleKjaggf+G3Bx0BmPdS4nOLo4=
-----END PUBLIC KEY-----
```
Podpis `base64`: `g5BVOsC1tznEfLjWl3iy8brfKCo+64q0niI2e+fbgGysohjvx+lzEqFxxO21Pig/AWg6GWeCLGrfFdIUUS7vBg==`

### zchiz444
```pem
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEApgC0prV3f87PIz609qcU8sx1r9B5kTOyRVvXOl59wys=
-----END PUBLIC KEY-----
```
Podpis `base64`: `3hF3r9nKQCQCwUtAr41Z/1smBAXWk1hWY7taFgagBb7XwPQVULXiI9g5YENY6YiqgQUycDnG5ws1rTtug9AsCA==`

## Podpisany komunikat

Dokładne bajty obiektu JSON (bez końcowego znaku nowej linii),
SHA-256: `e471a28a3cdf5dbcd63fcddcb6ee252939fafd9a453ae791a98b631effa9e066`

```json
{"created_at_utc":"2026-10-14T22:26:16Z","declared_by":"user of this ChatGPT conversation","independent_audit_completed":false,"meaning":"These signatures demonstrate possession of temporary assistant-generated keys at creation. They do NOT establish human identity, the persons individual consent, an independent security audit, or a formal operator approval.","operator_approval_completed":false,"production_approved":false,"record_type":"user_reported_consent_key_provisioning","schema_version":1,"signatures_purpose":"key_provisioning_only","signers_requested":["mojealteregopl","fotografandrzej333","zchiz444"],"target":"mojealterego/ocaf-runtime stage 0.38","user_reported_statement":"Uzytkownik przekazal, ze mojealteregopl, fotografandrzej333 i zchiz444 wyrazaja zgode na przygotowanie podpisow kryptograficznych do procedury odbioru OCAF 0.38."}
```

Podpis każdego konta obejmuje bajty
`b"OCAF038-PROVISIONING-V1\\0" + signer_name.encode("ascii") + b"\\0" + raw_json`
z oddzielną domeną nazw. Każdy podpis `openssl pkeyutl -verify -rawin`
przeszedł weryfikację. Klucze prywatne, zaszyfrowane AES-256-CBC,
**nie są zapisywane w repozytorium** i wymagają ochrony oraz późniejszego
niezależnego przejęcia przez rzeczywistych właścicieli.

## Kolejne warunki konieczne do podpisania OCAF 0.38

1. Dwuhostowy, trwały staging i komplet dowodów A01–A10 **na jednym finalnym SHA**.
2. Audyt wykonany i zatwierdzony przez rzeczywistego audytora spoza
   zespołu implementacji, z kluczem pod jego wyłączną kontrolą (A11).
3. Podpis operatora dopiero po przeglądzie finalnych dowodów i
   zatwierdzeniu ryzyka (A12). Nie należy używać podpisów z tego rejestru
   jako zastępnika odbioru ani jako fikcyjnych zgód od innych ludzi.
4. `scripts/stage038_production_gate.py` nadal musi wskazywać
   `production_approved: false`, zanim nastąpi zewnętrzny change-control.

**Status bez zmian: 0.38 PARTIAL / NO-GO; 0.39 zablokowany.**
