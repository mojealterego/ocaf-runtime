"""Regression: historical implemented OCAF development work must not be erased by blanket TODO."""
import copy
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import render_readme_roadmap as renderer

ROOT=Path(__file__).resolve().parents[2]

class RecoveredImplementationStatusTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inv=json.loads((ROOT/"docs/roadmap/canonical_v11_inventory.json").read_text())
        cls.builders=json.loads((ROOT/"docs/roadmap/agents_builder_products_v11.json").read_text())

    def row(self,stage):
        return next(r for r in self.inv["main"] if r["id"]==stage)

    def test_real_historical_code_implementation_not_all_todo(self):
        for stage in ("0.06","0.07","0.08","0.09","0.11","0.12","0.13",
                      "0.14","0.15","0.16","0.17","0.18","0.19","0.20",
                      "0.21","0.22","0.23","0.24","0.25","0.26","0.27",
                      "0.28","0.29","0.30","0.31","0.32","0.33","0.34","0.35"):
            with self.subTest(stage=stage):
                row=self.row(stage)
                self.assertIn(stage,renderer.HISTORICAL_IMPLEMENTED_SCOPES)
                self.assertEqual(renderer.mark(row["status"],"main",row),
                                 "ZROBIONE — ZAKRES IMPLEMENTACJI")

    def test_missing_proof_is_not_pass_and_no_prod_claim(self):
        row=copy.deepcopy(self.row("0.11"))
        row["id"]="0.10"
        self.assertEqual(renderer.mark(row["status"],"main",row),"DO ZROBIENIA")
        for stage in ("0.00","0.01","0.02","0.03","0.04","0.05","0.38","0.39","0.40","0.41","0.50","0.51","0.52","0.53","0.54","0.55"):
            self.assertNotEqual(renderer.mark(self.row(stage)["status"],"main",self.row(stage)),
                                "ZROBIONE — ZAKRES IMPLEMENTACJI")

    def test_sources_actually_present(self):
        mapping={
            "0.06":["src/jobs/mod.rs","tests/durable_jobs.rs"],
            "0.07":["src/coordinator/mod.rs","tests/coordinator.rs"],
            "0.08":["src/jobs/remote.rs","tests/distributed_jobs_rpc.rs"],
            "0.09":["src/coordinator/leadership.rs","tests/coordinator_leadership.rs"],
            "0.11":["src/cluster/leadership.rs","tests/postgres_coordinator_state.rs"],
            "0.12":["src/cluster/jobs.rs","tests/postgres_audit.rs"],
            "0.13":["src/bin/ocaf-cluster-health.rs","tests/cluster_observability.rs"],
            "0.14":["src/bin/ocaf-rpc-node.rs","tests/zenoh_chaos_takeover.rs"],
            "0.15":["src/rpc/cert_identity.rs","tests/zenoh_mtls_e2e.rs"],
            "0.16":["src/cluster/jobs.rs","tests/postgres_lease_clock.rs"],
            "0.17":["src/cluster/leadership.rs","tests/postgres_leadership.rs"],
            "0.18":["src/cluster/jobs.rs","tests/postgres_worker_fencing.rs"],
            "0.19":["src/cluster/jobs.rs","tests/postgres_fenced_effect.rs"],
            "0.20":["src/rpc/mod.rs","tests/postgres_cluster_execute.rs"],
            "0.21":["src/bin/ocaf-effect-sink.rs","tests/independent_sink.rs"],
            "0.22":["src/transport/sink.rs","tests/sink_identity.rs"],
            "0.23":["src/cluster/leadership.rs","tests/postgres_ha_recovery.rs"],
            "0.24":["tests/postgres_network_chaos.rs","tests/zenoh_chaos_takeover.rs"],
            "0.25":["src/bin/ocaf-cluster-health.rs","tests/cluster_health_cli.rs"],
            "0.26":["tests/final_security_acceptance.rs","tests/final_replay_acceptance.rs"],
            "0.27":["scripts/release_readiness.py","scripts/tests/test_release_readiness.py"],
            "0.28":["scripts/supply_chain.py","scripts/tests/test_supply_chain.py"],
            "0.29":["docs/SUPPLY_CHAIN_FINDINGS.md","tests/zenoh_mtls_e2e.rs"],
            "0.30":["Cargo.lock","docs/ZENOH_MIGRATION_030.md"],
            "0.31":[".github/workflows/supply-chain.yml","docs/SUPPLY_CHAIN.md"],
            "0.32":["docs/RELEASE_EVIDENCE_032.md",".github/workflows/supply-chain.yml"],
            "0.33":["scripts/ha_multihost_acceptance.py","scripts/tests/test_ha_multihost_acceptance.py"],
            "0.34":["scripts/mtls_lifecycle.py","scripts/tests/test_mtls_lifecycle.py"],
            "0.35":["tests/rpc_key_rotation.rs","src/rpc/mod.rs"],
        }
        self.assertEqual(set(mapping),renderer.HISTORICAL_IMPLEMENTED_SCOPES)
        for stage,paths in mapping.items():
            for path in paths:
                with self.subTest(stage=stage,path=path):
                    self.assertTrue((ROOT/path).is_file(),path)

    def test_front_pinned_readme_contains_historical_implementation_labels(self):
        section=renderer.render_section(self.inv,self.builders)
        self.assertIn("7. **[ZROBIONE — ZAKRES IMPLEMENTACJI]**",section)
        self.assertIn("12. **[ZROBIONE — ZAKRES IMPLEMENTACJI]**",section)
        self.assertIn("36. **[ZROBIONE — ZAKRES IMPLEMENTACJI]**",section)
        self.assertIn("50. **[DO ZROBIENIA]**",section)
        self.assertEqual(section.count("ZROBIONE — ZAKRES IMPLEMENTACJI]"),29)

if __name__=="__main__":
    unittest.main()
