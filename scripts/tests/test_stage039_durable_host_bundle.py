import copy
import hashlib
import json
import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage039_durable_host_bundle import BundleDenied, compare, validate_soak


def soak(tenant, boot):
    chain = bytes(32)
    cycles = []
    for i in range(1, 101):
        digest = hashlib.sha256(f"{tenant}/{boot}/{i}".encode()).hexdigest()
        chain = hashlib.sha256(chain + bytes.fromhex(digest)).digest()
        cycles.append({"cycle": i, "vm_id": f"vm-{i}",
                       "vm_report_sha256": digest, "rootfs_fd_inode": i,
                       "cgroup_drained_after_sigkill": True})
    return {"status": "ONE_HOST_100_REAL_KVM_JAILED_VM_SIGKILL_CYCLES_PASS",
            "tenant_id": tenant, "host_boot_id": boot,
            "cycle_count": 100, "distinct_vm_ids": 100,
            "all_running_rootfs_fd_inodes_pinned": True,
            "all_vm_cgroups_drained_after_sigkill": True,
            "durable_host_identity_proved": False,
            "receipt_chain_sha256": chain.hex(), "cycles": cycles}


def report(tenant, machine, product, boot):
    s = soak(tenant, boot)
    raw = json.dumps(s, sort_keys=True, indent=2).encode() + b"\n"
    return {"schema_version": 1, "status": "HOST_COLD_BOOT_SOAK_RECEIPT_CAPTURED",
            "tenant_id": tenant, "machine_id": machine,
            "dmi_product_uuid": product, "host_boot_id": boot,
            "soak_sha256": hashlib.sha256(raw).hexdigest(), "soak": s,
            "host_identity_cryptographically_attested": False,
            "interhost_partition_tested": False,
            "stage_039_complete": False, "production_approved": False}


class DurablePairTests(unittest.TestCase):
    def setUp(self):
        self.reports = [
            report("tenant-a", "a" * 32, "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                   "11111111-1111-1111-1111-111111111111"),
            report("tenant-b", "b" * 32, "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                   "22222222-2222-2222-2222-222222222222"),
            report("tenant-a", "a" * 32, "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                   "33333333-3333-3333-3333-333333333333"),
            report("tenant-b", "b" * 32, "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                   "44444444-4444-4444-4444-444444444444")]

    def test_four_boot_pair_report_remains_unapproved(self):
        result = compare(self.reports)
        self.assertEqual(result["total_jail_vm_cycles"], 400)
        self.assertFalse(result["host_identity_cryptographically_attested"])
        self.assertFalse(result["interhost_partition_tested"])
        self.assertFalse(result["stage_039_complete"])

    def test_reused_boot_or_cloned_identity_denied(self):
        for key in ("host_boot_id", "machine_id", "dmi_product_uuid"):
            records = copy.deepcopy(self.reports)
            records[2][key] = (records[0][key] if key == "host_boot_id"
                               else records[1][key])
            with self.subTest(key=key), self.assertRaises(BundleDenied):
                compare(records)

    def test_missing_cycle_and_mutated_chain_denied(self):
        for mutation in ("missing", "digest", "drain"):
            records = copy.deepcopy(self.reports)
            cycle = records[0]["soak"]["cycles"]
            if mutation == "missing": cycle.pop()
            elif mutation == "digest": cycle[0]["vm_report_sha256"] = "f" * 64
            else: cycle[0]["cgroup_drained_after_sigkill"] = False
            with self.subTest(mutation=mutation), self.assertRaises(BundleDenied):
                compare(records)

    def test_forged_approval_denied(self):
        records = copy.deepcopy(self.reports)
        records[0]["production_approved"] = True
        with self.assertRaises(BundleDenied):
            compare(records)


if __name__ == "__main__":
    unittest.main()
