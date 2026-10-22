# Etap 0.40 — kontrakt PostgreSQL HA

Dodano `src/postgres_ha_contract.rs`. Bramka wymaga trzech odrębnych
identyfikatorów hosta, boot ID i kernela, rzeczywistego TCP WAL streaming,
świadka fencing, odmowy zapisu starego primary oraz receiptów promocji i
recovery.

Kontrakt odrzuca wspólny kernel, transfer artefaktowy udający TCP, brak fencing
i brak recovery receipt. Status etapu pozostaje `ACTIVE_TECHNICAL_OPEN` do
czasu dowodu na rzeczywiście niezależnych hostach.
