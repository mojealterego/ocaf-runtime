"""TLS downgrade and distinct-role policy must fail closed before lab acceptance."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage040_pg_tls_lab as lab


class PgTlsPolicy(unittest.TestCase):
    def test_no_tcp_trust_or_password_fallback(self):
        rules = lab.hba_policy().splitlines()
        self.assertIn("hostnossl all all 127.0.0.1/32 reject", rules)
        self.assertIn("hostssl replication ocaf_repl 127.0.0.1/32 cert", rules)
        self.assertIn("hostssl all ocaf_admin 127.0.0.1/32 cert", rules)
        self.assertIn("host all all 0.0.0.0/0 reject", rules)
        self.assertFalse(any(line.startswith("host ") and
                             (" trust" in line or " password" in line or " md5" in line)
                             for line in rules))

    def test_missing_negative_or_overclaimed_host_rejected(self):
        report = {
            "status": "ONE_KERNEL_PG_TLS13_SEPARATE_ROLES_LAB_PASS",
            "tls13_admin_tcp_verified": True,
            "admin_client_certificate_verified": True,
            "replication_client_certificate_verified": True,
            "distinct_role_certificates": True,
            "replication_role_not_superuser": True,
            "admin_role_not_superuser": True,
            "admin_role_cannot_replicate": True,
            "replication_role_cannot_admin": True,
            "non_tls_tcp_rejected": True,
            "rogue_client_certificate_rejected": True,
            "wrong_ca_rejected": True,
            "one_underlying_linux_kernel": True,
            "independent_host_tls_tested": False,
            "pqc_hybrid_tested": False,
            "stage_040_complete": False,
            "production_approved": False,
        }
        self.assertIs(lab.validate_report(report), report)
        for field in ("non_tls_tcp_rejected", "rogue_client_certificate_rejected",
                      "wrong_ca_rejected", "admin_role_cannot_replicate"):
            with self.subTest(field=field), self.assertRaises(lab.PgTlsLabError):
                lab.validate_report({**report, field: False})
        for field in ("independent_host_tls_tested", "pqc_hybrid_tested",
                      "stage_040_complete", "production_approved"):
            with self.subTest(field=field), self.assertRaises(lab.PgTlsLabError):
                lab.validate_report({**report, field: True})


if __name__ == "__main__":
    unittest.main()
