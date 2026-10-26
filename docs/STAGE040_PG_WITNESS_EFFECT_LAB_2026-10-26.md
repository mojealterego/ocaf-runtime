# 0.40.8 / 0.40.12 — majority-gated PostgreSQL effect adapter lab

[GitHub Actions run 40282830351](https://github.com/mojealterego/ocaf-runtime/actions/runs/40282830351)
passed on commit `f62abbe6` with a real PostgreSQL postmaster. The lab admitted
two authenticated witness votes for candidate A, executed one SQL insert,
withheld the second vote and confirmed that the adapter did not invoke SQL,
then admitted a higher epoch for candidate B and confirmed that old A could
not invoke SQL through the adapter. PostgreSQL returned exactly the rows
`1:node-a` and `2:node-b`.

The [driver](../scripts/stage040_pg_witness_effect_lab.py) uses the durable
[majority certificate gate](../scripts/stage040_quorum_certificate.py). The
[three negative tests](../scripts/tests/test_stage040_pg_witness_effect_lab.py)
also cover candidate mismatch and injection-shaped input. A separate
[CI gate](../.github/workflows/stage040-pg-witness-effect-lab.yml) runs both
the tests and actual PostgreSQL process. GitHub reports artifact ID
`12632692603`, archive SHA-256
`edbc5f406cca318ccc678f42e4e17a86bc3f71409d34d1d73aa686b9298f1b26`.

This is **adapter-level** write admission on **one PostgreSQL postmaster**.
All three witness keys are generated in one laboratory process. Raw
PostgreSQL superuser clients can bypass the adapter, and certificate admission
and SQL execution are not one atomic cross-system transaction. The run does
not prove independent witness hosts, server-side fencing, partition-safe
three-host HA or stage completion. Its report sets
`postgresql_server_side_fencing_proved=false`,
`independent_witness_hosts_tested=false`, `three_host_ha_tested=false`, and
`stage_040_complete=false`.
