# 0.41 — przeciążenie lokalnej kolejki telemetrii

Moduł `src/metrics/bounded_telemetry.rs` udostępnia ograniczoną kolejkę
zdarzeń i jawny licznik odmów. Gdy odbiorca przestaje pobierać próbki,
producent dostaje `Backpressure`; po zamknięciu kanału dostaje `Closed`.
Po wznowieniu odbioru wciąż możliwe jest przesłanie nowych próbek.

Test E2E w obrębie procesu przeprowadza 60 minut rzeczywistej pracy
producenta i odbiorcy w trybie ręcznym workflow, mierzy przyjęte, odrzucone
i odebrane próbki oraz wymaga pełnego opróżnienia kolejki. Krótki przebieg
CI wykonuje ten sam kontrakt przez sekundę. Nie jest to eksport OTLP,
trwałość telemetryczna ani zintegrowane obciążenie brokera i VM. Nie należy
z niego wyciągać wniosku o statusie `stage_041_complete` lub produkcyjnym GO.
