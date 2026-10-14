# OCAF 0.38 — sześć nowych par kluczy Ed25519 (przekazanie / bez odbioru)

**Data technicznego utworzenia:** 2026-10-14T23:39:40Z (UTC).
**Repo:** `mojealterego/ocaf-runtime`; **branch:** `main`.
**Stan:** KEY PROVISIONING ONLY — `production_approved=false`, **0.38 NO-GO**.

Na życzenie użytkownika automat kryptograficzny utworzył **sześć odrębnych losowych par Ed25519**. Każdy prywatny klucz został zaszyfrowany jako PKCS#8 PEM indywidualnym losowym hasłem. Dla każdej pary wykonano i zweryfikowano podpis *wyłącznie technicznego komunikatu o wygenerowaniu klucza*. Sześć weryfikacji podpisu, odszyfrowania i SHA-256 kluczy publicznych zakończyło się sukcesem.

**Punkt odniesienia podpisu provisioning JSON, SHA-256:** `ada56f9516ea0afd978ef6053d246074ec5b6aa2c3f196111801bfb605441947`. Podpisy mają domenę `OCAF038-KEY-PROVISIONING-V2\0<identifier>\0<payload>` i nie są podpisami `OCAF038-ACCEPTANCE-V1`.

| Zgłoszony identyfikator | Rola deklarowana przez użytkownika | SHA-256 public DER |
|---|---|---|
| `andrzej_mikulski` | zgłoszony operator A12 | `3d5b740a431b91c3e6142083966f891456a079caa1b78e4558e72da78267a36a` |
| `zchiz444` | zgłoszony audytor A11 | `7a2056332cda5fcc19b641f03c569e03b8f6d4d08219ec6089573cd9fa5a3d3d` |
| `arminburen84` | zgłoszony audytor A11 | `e8db4461b5a3e7ab31f9721584a122e09c49d3dd1674d9b854d3b71996688c77` |
| `wieslawdeskareporter` | zgłoszony audytor A11 | `59b8eeb4038f2251fd939de7cc8b83f6461035b2d1ddd996f6520d600e7bbbf8` |
| `mojealteregopl` | uczestnik zgłoszony | `ae21bb3655059a505f6bc8feafe50f95a0bbb5d80e666c731c5b44224edb69b1` |
| `fotografandrzej333` | uczestnik zgłoszony | `3afbc531224614214689a654fd74cc58a31b3150e147b6fdfb1c7d5ad96c9577` |

## Publiczne klucze Ed25519

### andrzej_mikulski

```pem
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAv8CDb+dB+EVU8f+hwMCAu58MRSSOrANwG32EpxrxBR8=
-----END PUBLIC KEY-----
```

### zchiz444

```pem
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAWiAaFTLngMkURy/Fw03rsjTdPRYlUevyDuuVZa18lIQ=
-----END PUBLIC KEY-----
```

### arminburen84

```pem
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEACly+n3l5R7WODoLGwLlCH8JtasaV75ESINhUfoC16wM=
-----END PUBLIC KEY-----
```

### wieslawdeskareporter

```pem
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAw6O2PW+o1/B41etqLhDkKWvTs7dF1TtIdFsTHp1xTqo=
-----END PUBLIC KEY-----
```

### mojealteregopl

```pem
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAtcg+a5ruIuwW8Gf8BAaqiCgiQlklS/JM7av2AHsUCxQ=
-----END PUBLIC KEY-----
```

### fotografandrzej333

```pem
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAFgPFZC9df/0lKAwe+bflsrKHr19lDkJhhqGVDniOcA8=
-----END PUBLIC KEY-----
```

## Niezależność i granica podpisów

Pary kluczy utworzył **jeden automat**, a nie osobne osoby w niezależnych środowiskach. Rejestracja identyfikatora i techniczna sygnatura potwierdzają jedynie dysponowanie kluczem przez generator. **Nie stanowią weryfikacji tożsamości, zgody każdej osoby z osobna, osobistego audytu bezpieczeństwa ani odbioru operatora.**

**Żaden prywatny klucz ani hasło nie zostały umieszczone w tym repozytorium.** Przekazanie zaszyfrowanych kluczy i oddzielnych haseł odbywa się wyłącznie poza publicznym repozytorium. Każdy rzeczywisty właściciel powinien sam przejąć kontrolę i najlepiej wygenerować docelowy klucz we własnym środowisku.

Nie umieszczać podpisów provisioning jako `auditor.sig` / `operator.sig`. Wymagane są dwa trwałe niezależne hosty, rzeczywisty chaos i dowody A01–A10 oraz audyt i odbiór A11–A12 wykonane przez uprawnione osoby. **Status nadal 0.38 PARTIAL / NO-GO, 0.39 zablokowany.**
