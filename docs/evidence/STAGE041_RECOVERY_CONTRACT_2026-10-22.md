# Stage 0.41 — recovery and chaos contract

Implemented `src/stage041_recovery_contract.rs` as a fail-closed evidence
boundary for RTO/RPO, WAL replay, backup restore, fencing and power-loss
recovery.

The contract rejects a local copy presented as an independent backup, requires
distinct failure domains, requires a verified restore and rejects observed RTO
or RPO above the declared target. The four unit tests cover acceptance and
each negative boundary.

Status: `ACTIVE_TECHNICAL_OPEN` until the real multi-host chaos run, restore
receipt and Rust build are executed. This file is not a production acceptance.
