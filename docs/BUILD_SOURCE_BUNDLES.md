# OCAF — paczki źródłowe ZIP Agents Builder

## Dwa niezależne archiwa

1. **ocaf-agents-builder-<SHA12>.zip** — komplet plików implementacyjnych
   wspólnego GUI Code/NoCode i przełącznika Local/Cloud, języków, kluczy API
   w pamięci bieżącej sesji, profili GGUF/Hugging Face i ich testów; ponadto
   moduły Rust builder_core/store/cloud/http, model_router, kontrakty OAuth,
   dokumentacja wdrożeniowa, kanoniczne pliki roadmapy oraz workflow CI.
2. **ocaf-runtime-full-source-<SHA12>.zip** — wszystkie pliki regularne
   śledzone przez Git w źródłowym commicie OCAF, nie tylko Agents Builder.
   Nie obejmuje katalogu .git, plików nieśledzonych, sekretów środowiska,
   plików modeli GGUF pobranych przez użytkownika ani buildów binarnych.

Każde archiwum zawiera RELEASE_MANIFEST.json. Manifest wymienia **wszystkie**
pliki paczki, rozmiary i SHA-256, hash źródłowego commita oraz stan akceptacji.
Zewnętrzny SHA256SUMS umożliwia weryfikację plików ZIP.

## Tworzenie i testowanie

Z głównego katalogu repozytorium, z czystego HEAD:

\`\`\`bash
python3 -m unittest discover -s scripts/tests -p test_package_agents_builder.py -v
node --test apps/agents-builder-web/tests/*.test.mjs
python3 scripts/package_agents_builder.py --repo . --out dist
cd dist && sha256sum -c SHA256SUMS
\`\`\`

GitHub Actions: \`.github/workflows/agents-builder-source-bundle.yml\`.
Workflow sprawdza testy bezpieczeństwa pakowania, testy JavaScript i
przeglądarkowy GUI E2E, wymusza SHA i czyste źródła, generuje dwa deterministyczne
archiwa, weryfikuje CRC i SHA-256 każdego wpisu, publikuje artefakty w Actions
oraz tworzy **prerelease**, zawierający pliki ZIP do bezpośredniego pobrania.

Wydania źródłowe są oznaczane tagiem
\`ocaf-agents-builder-source-<SHA12>\` i dostępne na stronie **Releases**
w repozytorium. Nie oznacza to kompletnego produktu produkcyjnego.

## Uruchomienie po pobraniu

W paczce Agents Builder plik \`apps/agents-builder-web/index.html\` otwiera się
po wystawieniu katalogu \`apps/agents-builder-web\` przez lokalny HTTP server:

\`\`\`bash
python3 -m http.server 8765 --bind 127.0.0.1 --directory apps/agents-builder-web
\`\`\`

Następnie otwórz \`http://127.0.0.1:8765/\` na tym samym urządzeniu.
Zapisane profile GGUF są konfiguracją. Faktyczna inferencja wymaga osobnego,
wspieranego runnera, a Cloud — uwierzytelnionego gateway z OAuth.
Klucze API **nie są** umieszczane w paczkach ani eksportach.

Status odbioru: \`TESTED_LOCAL_PARTIAL\`. Nie nadano \`DEPLOYED\`,
\`OPERATOR_ACCEPTED\` ani \`PRODUCTION_APPROVED\`.

## Inference GGUF — dodatkowe pliki pakietu

Pakiet Agents Builder zawiera teraz również skrypt lokalnego runnera Python, klienta JS, testy subprocess/HTTP i dokumentację. Źródłowe wydanie ZIP nie zawiera skompilowanego llama-server ani żadnych wag GGUF. Operator instaluje binarkę zgodną z platformą i podaje katalog modeli. Szczegóły i polecenie uruchomienia: apps/agents-builder-web/README.md.
