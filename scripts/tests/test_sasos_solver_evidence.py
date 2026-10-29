"""Stage 334 solver evidence gate: result parser and real Z3 integration."""
from pathlib import Path
import shutil
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from sasos_solver_evidence import SolverEvidenceError, check_models, verify_result


class SolverEvidenceTests(unittest.TestCase):
    def test_rejects_unknown_sat_when_unsat_expected_and_multiple_answers(self):
        for value in ("unknown", "sat", "unsat\nsat", "", "(error \"bad\")"):
            with self.subTest(value=value), self.assertRaises(SolverEvidenceError):
                verify_result(value, "unsat")

    def test_missing_solver_fails_closed(self):
        with self.assertRaises(SolverEvidenceError):
            check_models("ocaf-sasos334-nonexistent-solver")

    @unittest.skipUnless(shutil.which("z3"), "Z3 installed by dedicated CI job")
    def test_actual_z3_runs_all_models_and_negative_control(self):
        report = check_models()
        self.assertEqual(report["status"], "BOUNDED_SMT_MODEL_CHECKED_NOT_RUST_FORMAL_PROOF")
        self.assertEqual([case["result"] for case in report["cases"]],
                         ["unsat", "unsat", "unsat", "sat"])
        self.assertFalse(report["rust_implementation_formally_verified"])
        self.assertFalse(report["imandra_verified"])


if __name__ == "__main__":
    unittest.main()
