# OCAF 40 PDF — kontrola atomizacji i bramki audytu

Przeanalizowano 40 dostępnych PDF / 902 strony i wyodrębniono **4123 fragmenty tekstowe** zawierające terminy architektoniczne. Spośród nich 148 oznaczono jako potencjalnie normatywne prostym filtrem językowym; 3975 jako opisowe/referencyjne. **Te liczby nie są liczbą rzeczywistych wymagań.** Wiele wymagań nie zawiera słów kluczowych, a fragmenty opisowe mogą zawierać wymagania. Każdy rekord zachowuje: numer dokumentu, nazwę, stronę, fragment źródła, SHA-256 fragmentu, kategorię, proponowane identyfikatory etapów i flagę `NEEDS_HUMAN_SEMANTIC_VERIFICATION`.

Lokalne artefakty: `OCAF_40_PDF_ATOMIC_CANDIDATES.csv`, `OCAF_40_PDF_ATOMIC_CANDIDATES.json`, `OCAF_40_PDF_ATOMIC_CANDIDATES_SUMMARY.md`. Repozytorium nie przechowuje tych lokalnych plików automatycznie.

## Warunki odbioru — obecnie NIEZALICZONE

1. Weryfikacja każdego fragmentu na stronie źródłowej i odróżnienie nakazu projektowego od przeglądu literatury.
2. Dalszy podział zdań wielowarunkowych na atomowe wymagania, wraz z zależnościami i testami.
3. Pełne semantyczne porównanie z 281 pozycjami Roadmapy (207 głównych, 64 techniczne, 10 produktów).
4. Aktualizacja manifestu JSON i obu kopii Roadmapy dla wszystkich potwierdzonych braków.
5. Testy CI i potwierdzenie stanu kodu niezależnie od statusów dokumentacyjnych.

**Wniosek:** obecny korpus przeszedł ekstrakcję i indeksowanie, ale nie pełną weryfikację semantyczną. Wcześniejsze określenia „sprawdzono wszystkie strony” należy rozumieć jako odczyt/klasyfikację, nie dowód kompletności Roadmapy.