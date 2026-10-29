# 0.43 — trwały rejestr OMAt przez Zenoh

Osobny proces `ocaf-omat-zenoh` wymaga istniejącej bazy
`OCAF_OMAT_TOOL_DB`, prywatnego pliku `OCAF_OMAT_POLICY_JSON`,
`OCAF_OMAT_REPLAY_DB`, `OCAF_OMAT_PEER`, `OCAF_OMAT_TENANT`,
`OCAF_OMAT_SHARED_KEY_B64`, `OCAF_OMAT_REVOKE_SENDER` oraz
`OCAF_OMAT_REVOKE_PUBLIC_KEY_B64`. Brak konfiguracji powoduje odmowę startu.
Konfiguracja Zenoh nadal musi przejść istniejącą kontrolę TLS.

Wcześniej podpisane manifesty są odczytywane z bazy przy odkrywaniu i
pobraniu schematu. Odpowiedź `ocaf/omat/{tenant}/discovery` zawiera
identyfikator, wersję i skrót manifestu, bez nieufnego opisu jako instrukcji.
Pobranie schematu wymaga dokładnego skrótu. Każde żądanie ma HMAC,
przypięty temat, nadawcę i trwały nonce w SQLite. Rejestr jest odczytywany
ponownie po odwołaniu, więc nowy request nie otrzymuje tombstone.

Odwołanie przyjmuje tylko osobna trasa
`ocaf/omat/{tenant}/control/revoke` z podpisem Ed25519 v2. Nadawca
operatora i jego klucz publiczny pochodzą z konfiguracji procesu, nie z ramki.
Nonce i tombstone są trwałe. OMAt nadal odmawia wykonania narzędzi, ponieważ
OSTc jest wyłączony i agent nie może przejść do ACTIVE.

Test obejmuje 550 podpisanych manifestów, Zenoh queryable, tenant isolation,
odmowę replay, zły HMAC, zły digest schematu, ukrycie złośliwego opisu,
oraz odwołanie widoczne w kolejnym discovery. Lokalna baza SQLite jest
obszarem zaufania operatora. Nie ma tu dowodu rozproszonego klastra,
certyfikacji, benchmarku produkcyjnego ani wykonania narzędzia.

## Katalog SASOS 001–555

`scripts/import_sasos_tool_catalog.py` przyjmuje kompletny plik źródłowy Rust
z definicjami `SasosTool001`–`SasosTool555` i wywołaniami `tools.insert`.
Sprawdza ciągłość numeracji, unikalność ID oraz zgodność wpisów rejestru
z definicjami. Polecenie:

```sh
python3 scripts/import_sasos_tool_catalog.py katalog.rs katalog.json
```

Wynik jest katalogiem metadanych z `status: CATALOG_ONLY` oraz
`executable: false`. Deklaracje `SecurityClearance` i `execution_runtime`
są zachowane jako tekst, a nie stosowane jako uprawnienia lub adaptery.
Wartości demonstracyjne `imandra_verified: true` i `latency_microseconds: 7`
z metody domyślnej nie stanowią dowodu; eksport podaje `false` i `null`.
Importer nie wykonuje kodu Rust ani żadnego narzędzia. Do aktywacji potrzebne
są osobne implementacje, podpisane manifesty i kontrole OSTc.

Katalog z pliku Drive `ocaf_runtime_omat_555_production_v2.rs` został
zweryfikowany i zapisany w `omat/catalog/sasos_tools_001_555.json`.
SHA-256 pobranego źródła:
`b4e9797990e827dba7113ab7f28fcb532f1bb058cf822bee6324d95f33e9df18`.
SHA-256 eksportu JSON:
`52635fe8e35f47fc394351c6620cb970646820b4967b061e3c6d09dc4410482a`.
W tym katalogu 316–320 to kolejno R3Mem, Titans, ImandraX, MARS i CWM.
Późniejszy fragment „OMAt Kernel Core + MARS 5-Step Pipeline” ponownie
używa tych numerów dla innych funkcji, więc wymaga osobnej decyzji o wersji
lub nowych identyfikatorów przed rejestracją adapterów.
