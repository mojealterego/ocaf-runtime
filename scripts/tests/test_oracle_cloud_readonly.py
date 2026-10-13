"""Offline contract tests; NEVER call an OCI account or paid service."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / 'oracle_cloud_readonly.py'
SPEC = importlib.util.spec_from_file_location('ocaf_oracle_cloud_readonly', SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

TENANCY = 'ocid1.tenancy.oc1..examplevalid123'
COMPARTMENT = 'ocid1.compartment.oc1..examplevalid123'


def fake_oci(args):
    path = tuple(args[:3])
    if path == ('iam', 'region-subscription', 'list'):
        return {'data': [{'region-name': 'eu-frankfurt-1', 'is-home-region': True},
                         {'region-name': 'us-ashburn-1', 'is-home-region': False}]}
    if path == ('compute', 'instance', 'list'):
        return {'data': [
            {'shape': 'VM.Standard.A1.Flex', 'lifecycle-state': 'RUNNING',
             'shape-config': {'ocpus': 1.0, 'memory-in-gbs': 6.0},
             'display-name': 'USER_PRIVATE_NAME', 'id': 'ocid1.instance.oc1.USER_PRIVATE'},
            {'shape': 'VM.Standard.E2.1.Micro', 'lifecycle-state': 'STOPPED'}]}
    if path == ('network', 'vcn', 'list'):
        return {'data': [{'display-name': 'SECRET_LAN', 'id': 'ocid1.vcn.oc1.USER_PRIVATE'}]}
    if path == ('budgets', 'budget', 'budget'):
        return {'data': [{'display-name': 'PERSONAL_BUDGET', 'amount': 2}]}
    raise AssertionError('unexpected OCI API: ' + str(args))


class OracleReadOnlyTests(unittest.TestCase):
    def test_default_plan_is_within_always_free_capacity(self):
        result = MODULE.build_plan(2, 1, 6, 50)
        self.assertEqual(result['instances'], 2)
        self.assertEqual(result['allocated_a1_ocpu'], 2)
        self.assertEqual(result['allocated_a1_memory_gb'], 12)
        self.assertEqual(result['boot_gb'], 100)
        self.assertEqual(result['provisioning'], 'disabled')

    def test_rejects_overprovisioning_cpu_ram_boot(self):
        for args in ((2, 2, 6, 50), (2, 1, 8, 50), (2, 1, 6, 101), (0, 1, 6, 50), (3, 1, 6, 50)):
            with self.subTest(args=args), self.assertRaises(ValueError):
                MODULE.build_plan(*args)

    def test_rejects_invalid_ocids_and_regions(self):
        with self.assertRaises(ValueError):
            MODULE.validate_id('ocid1.user.oc1..wrong', 'tenancy')
        with self.assertRaises(ValueError):
            MODULE.validate_id('abc; rm -rf /', 'compartment')
        with self.assertRaises(ValueError):
            MODULE.validate_region('http://bad?token=x')

    def test_oci_commands_are_explicitly_read_only(self):
        calls = []
        def runner(args):
            calls.append(args)
            return fake_oci(args)
        report = MODULE.inspect(TENANCY, COMPARTMENT, 'eu-frankfurt-1', runner)
        self.assertEqual(len(calls), 4)
        for args in calls:
            self.assertIn('list', args)
            self.assertNotIn('create', args)
            self.assertNotIn('update', args)
            self.assertNotIn('launch', args)
            self.assertNotIn('delete', args)
            self.assertIn('--region', args)
        self.assertEqual(report['home_region'], 'eu-frankfurt-1')
        self.assertEqual(report['instance_count'], 2)
        self.assertTrue(report['billing_upgrade_requires_manual_confirmation'])

    def test_inventory_redacts_resource_ids_and_names(self):
        report = MODULE.inspect(TENANCY, COMPARTMENT, 'eu-frankfurt-1', fake_oci)
        s = json.dumps(report)
        for private in ('USER_PRIVATE', 'SECRET_LAN', 'PERSONAL_BUDGET', TENANCY, COMPARTMENT):
            self.assertNotIn(private, s)
        self.assertEqual(report['running_instances'], 1)
        self.assertEqual(report['a1_ocpus_reported'], 1)

    def test_fails_closed_when_api_fails_or_malformed(self):
        def fails(args):
            raise MODULE.ReadError('OCI query failed')
        with self.assertRaises(MODULE.ReadError):
            MODULE.inspect(TENANCY, COMPARTMENT, 'eu-frankfurt-1', fails)
        def malformed(args):
            return {'data': {'this': 'is not a list'}}
        with self.assertRaises(MODULE.ReadError):
            MODULE.inspect(TENANCY, COMPARTMENT, 'eu-frankfurt-1', malformed)

    def test_cli_only_plan_and_inspect_modes(self):
        parser = MODULE.make_parser()
        with self.assertRaises(SystemExit):
            parser.parse_args(['upgrade'])
        with self.assertRaises(SystemExit):
            parser.parse_args(['pay'])
        with self.assertRaises(SystemExit):
            parser.parse_args(['deploy'])

    def test_report_created_private_once_no_overwrite(self):
        with tempfile.TemporaryDirectory() as temp:
            dest = Path(temp) / 'report.json'
            MODULE.save_report(dest, {'safe': True})
            self.assertEqual(json.loads(dest.read_text()), {'safe': True})
            self.assertEqual(dest.stat().st_mode & 0o777, 0o600)
            with self.assertRaises(FileExistsError):
                MODULE.save_report(dest, {'safe': False})

    def test_offline_plan_runs_with_no_credentials(self):
        with tempfile.TemporaryDirectory() as temp:
            report = Path(temp) / 'plan.json'
            child = subprocess.run([sys.executable, str(SCRIPT), 'plan', '--output', str(report)],
                capture_output=True, text=True, env={'PATH': os.environ.get('PATH', '')})
            self.assertEqual(child.returncode, 0, child.stderr)
            self.assertTrue(report.is_file())
            self.assertIn('manual_only', report.read_text())

    def test_inspect_mode_needs_explicit_tenancy_and_compartment(self):
        proc = subprocess.run([sys.executable, str(SCRIPT), 'inspect'],capture_output=True,text=True)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn('required', proc.stderr.lower())


if __name__ == '__main__':
    unittest.main()
