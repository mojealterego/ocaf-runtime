"""Real loopback OTLP transport plus fail-closed KVM evidence binding."""
from __future__ import annotations

import copy
import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage039_otlp_evidence as otlp
import stage039_traceparent as traceparent


def fixture():
    traces = {key: traceparent.derive(("a" if key == "a" else "b") * 32,
                                            ("c" if key == "a" else "d") * 64,
                                            "tenant-" + key) for key in ("a", "b")}
    broker = {
        "stage_039_complete": False,
        "broker_vm_binding": {
            "status": "BROKER_SO_PEERCRED_TWO_REAL_JAILED_VM_BOOT_BOUND",
            "real_kernel_so_peercred_authenticated_broker": True,
            "signed_wasi_manifest_verified": True,
            "guest_traceparent_propagated": True,
            "tenant_traceparents": traces,
        },
    }
    guest = {
        "stage_039_complete": False,
        "status": "TWO_REAL_BROKERED_JAILED_FIRECRACKER_GUESTS_WASIP1_FD_WRITE_EXECUTED",
        "real_guest_wasi_execution_verified": True,
        "signed_wasi_manifest_verified": True,
        "two_independent_hosts": False,
        "tenant_receipts": {},
    }
    for key in ("a", "b"):
        receipt = {"vm_id": "ocaf-" + ("a" if key == "a" else "b") * 24,
                   "tenant_id": "tenant-" + key, "traceparent": traces[key],
                   "task_nonce": ("e" if key == "a" else "f") * 32,
                   "module_sha256": ("1" if key == "a" else "2") * 64,
                   "serial_sha256": ("3" if key == "a" else "4") * 64,
                   "guest_openat_event_count": 7, "guest_openat_bpf_program_id": 3,
                   "guest_wasmtime_child_exited_zero": True,
                   "observed_before_vm_sigkill": True}
        guest["tenant_receipts"][key] = receipt
        broker["tenant_" + key] = {
            "tenant_id": receipt["tenant_id"], "vm_id": receipt["vm_id"],
            "traceparent": traces[key],
            "wasi": {"task_nonce": receipt["task_nonce"],
                     "module_sha256": receipt["module_sha256"]},
            "guest_wasi_execution": copy.deepcopy(receipt),
        }
    return broker, guest


class OtlpEvidenceTests(unittest.TestCase):
    def test_actual_http_export_and_collector_receive(self):
        broker, guest = fixture()
        payload, proof = otlp.build_export(broker, guest, 123456789)
        self.assertEqual(len(payload["resourceSpans"]), 2)
        self.assertNotEqual(proof["a"]["traceparent"], proof["b"]["traceparent"])
        result = otlp.export_to_loopback(payload)
        self.assertTrue(result["otlp_http_json_posted"])
        self.assertTrue(result["loopback_collector_received"])
        self.assertEqual(len(result["collector_body_sha256"]), 64)

    def test_mutated_guest_or_lease_trace_denied_before_export(self):
        base_broker, base_guest = fixture()
        for mutate in (
            lambda b, g: g["tenant_receipts"]["a"].update(traceparent=g["tenant_receipts"]["b"]["traceparent"]),
            lambda b, g: b["tenant_a"]["wasi"].update(module_sha256="0" * 64),
            lambda b, g: g["tenant_receipts"]["a"].update(guest_openat_event_count=0),
            lambda b, g: b["broker_vm_binding"].update(signed_wasi_manifest_verified=False),
            lambda b, g: b["tenant_b"].update(vm_id=b["tenant_a"]["vm_id"]),
        ):
            with self.subTest(mutate=mutate):
                broker, guest = copy.deepcopy(base_broker), copy.deepcopy(base_guest)
                mutate(broker, guest)
                with self.assertRaises(otlp.EvidenceError):
                    otlp.build_export(broker, guest, 123456789)


if __name__ == "__main__":
    unittest.main()
