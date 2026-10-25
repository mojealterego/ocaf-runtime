#!/usr/bin/env python3
"""Export verified stage 0.39 KVM evidence as OTLP/HTTP JSON to loopback.

These are evidence receipt spans exported after the VM run. They do not claim
that Wasmtime or the guest kernel emitted application telemetry directly.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import sys
import threading
import time
from urllib.request import Request, urlopen

import stage039_traceparent as traceparent

MAX_BODY = 64 * 1024


class EvidenceError(ValueError):
    pass


def _unique(pairs):
    obj = {}
    for key, value in pairs:
        if key in obj:
            raise EvidenceError("duplicate JSON evidence key")
        obj[key] = value
    return obj


def _read(path: Path) -> dict:
    raw = path.read_bytes()
    if not 0 < len(raw) <= 2 * 1024 * 1024:
        raise EvidenceError("missing or oversized KVM evidence")
    obj = json.loads(raw, object_pairs_hook=_unique,
                     parse_constant=lambda _: (_ for _ in ()).throw(EvidenceError("nonfinite JSON")))
    if not isinstance(obj, dict):
        raise EvidenceError("invalid KVM evidence object")
    return obj


def _attr(key: str, value: str | int) -> dict:
    return {"key": key, "value": {"intValue": str(value)} if type(value) is int
            else {"stringValue": value}}


def build_export(broker: dict, guest: dict, now_ns: int) -> tuple[dict, dict]:
    """Bind two receipt spans to broker, signed task, VM and guest events."""
    binding = broker.get("broker_vm_binding")
    if (not isinstance(binding, dict)
            or binding.get("status") != "BROKER_SO_PEERCRED_TWO_REAL_JAILED_VM_BOOT_BOUND"
            or binding.get("real_kernel_so_peercred_authenticated_broker") is not True
            or binding.get("signed_wasi_manifest_verified") is not True
            or binding.get("guest_traceparent_propagated") is not True
            or guest.get("status") != "TWO_REAL_BROKERED_JAILED_FIRECRACKER_GUESTS_WASIP1_FD_WRITE_EXECUTED"
            or guest.get("real_guest_wasi_execution_verified") is not True
            or guest.get("signed_wasi_manifest_verified") is not True
            or guest.get("two_independent_hosts") is not False
            or broker.get("stage_039_complete") is not False
            or guest.get("stage_039_complete") is not False
            or type(now_ns) is not int or now_ns <= 0):
        raise EvidenceError("verified one-host broker and guest receipts required")
    receipts = guest.get("tenant_receipts")
    parents = binding.get("tenant_traceparents")
    if (not isinstance(receipts, dict) or set(receipts) != {"a", "b"}
            or not isinstance(parents, dict) or set(parents) != {"a", "b"}
            or len(set(parents.values())) != 2):
        raise EvidenceError("two distinct tenant traces required")
    resource_spans = []
    proof = {}
    for key in ("a", "b"):
        vm = broker.get("tenant_" + key)
        receipt = receipts[key]
        parent = parents[key]
        if not traceparent.is_valid(parent) or not isinstance(vm, dict) or not isinstance(receipt, dict):
            raise EvidenceError("invalid VM trace binding")
        wasi = vm.get("wasi")
        observed = vm.get("guest_wasi_execution")
        if (not isinstance(wasi, dict) or not isinstance(observed, dict)
                or vm.get("tenant_id") != "tenant-" + key
                or vm.get("vm_id") != receipt.get("vm_id")
                or vm.get("traceparent") != parent
                or receipt.get("tenant_id") != vm["tenant_id"]
                or receipt.get("traceparent") != parent
                or observed.get("traceparent") != parent
                or wasi.get("task_nonce") != receipt.get("task_nonce")
                or wasi.get("module_sha256") != receipt.get("module_sha256")
                or observed.get("serial_sha256") != receipt.get("serial_sha256")
                or observed.get("guest_openat_event_count") != receipt.get("guest_openat_event_count")
                or observed.get("guest_openat_bpf_program_id") != receipt.get("guest_openat_bpf_program_id")
                or observed.get("guest_wasmtime_child_exited_zero") is not True
                or observed.get("observed_before_vm_sigkill") is not True
                or type(receipt.get("guest_openat_event_count")) is not int
                or receipt["guest_openat_event_count"] <= 0
                or type(receipt.get("guest_openat_bpf_program_id")) is not int
                or receipt["guest_openat_bpf_program_id"] <= 0):
            raise EvidenceError("guest VM, module, serial or openat receipt diverged")
        trace_id = bytes.fromhex(parent[3:35])
        span_id = bytes.fromhex(parent[36:52])
        span = {
            "traceId": base64.b64encode(trace_id).decode(),
            "spanId": base64.b64encode(span_id).decode(),
            "name": "ocaf.stage039.verified_guest_receipt",
            "kind": 1,
            "startTimeUnixNano": str(now_ns),
            "endTimeUnixNano": str(now_ns),
            "attributes": [
                _attr("ocaf.tenant_id", vm["tenant_id"]),
                _attr("ocaf.vm_id", vm["vm_id"]),
                _attr("ocaf.wasi_module_sha256", wasi["module_sha256"]),
                _attr("ocaf.guest_serial_sha256", receipt["serial_sha256"]),
                _attr("ocaf.guest_openat_event_count", receipt["guest_openat_event_count"]),
                _attr("ocaf.guest_openat_bpf_program_id", receipt["guest_openat_bpf_program_id"]),
                _attr("ocaf.timestamp_scope", "post_run_export_only"),
            ],
        }
        resource_spans.append({
            "resource": {"attributes": [_attr("service.name", "ocaf-stage039-evidence-bridge")]},
            "scopeSpans": [{"scope": {"name": "ocaf.stage039.evidence", "version": "1"},
                            "spans": [span]}],
        })
        proof[key] = {"vm_id": vm["vm_id"], "traceparent": parent,
                      "guest_serial_sha256": receipt["serial_sha256"]}
    if proof["a"]["vm_id"] == proof["b"]["vm_id"]:
        raise EvidenceError("reused VM ID")
    return {"resourceSpans": resource_spans}, proof


def export_to_loopback(payload: dict) -> dict:
    """Perform an actual OTLP/HTTP JSON POST and verify collector bytes."""
    body = json.dumps(payload, separators=(",", ":"), sort_keys=True).encode()
    if not 0 < len(body) <= MAX_BODY:
        raise EvidenceError("OTLP body outside bound")
    received = []

    class Receiver(BaseHTTPRequestHandler):
        def do_POST(self):
            try:
                length = int(self.headers.get("Content-Length", "-1"))
                if (self.path != "/v1/traces" or self.headers.get("Content-Type") != "application/json"
                        or not 0 < length <= MAX_BODY):
                    raise EvidenceError("invalid OTLP request")
                raw = self.rfile.read(length)
                parsed = json.loads(raw, object_pairs_hook=_unique)
                if not isinstance(parsed, dict) or set(parsed) != {"resourceSpans"}:
                    raise EvidenceError("invalid OTLP JSON body")
                received.append(raw)
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(b"{}")
            except (ValueError, OSError):
                self.send_error(400)

        def log_message(self, *_args):
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Receiver)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        url = f"http://127.0.0.1:{server.server_port}/v1/traces"
        request = Request(url, body, {"Content-Type": "application/json"}, method="POST")
        with urlopen(request, timeout=3) as response:
            if response.status != 200 or response.read(4096) != b"{}":
                raise EvidenceError("OTLP collector response denied")
        if len(received) != 1 or received[0] != body:
            raise EvidenceError("collector body diverged from exported spans")
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=3)
    return {"otlp_http_json_posted": True, "loopback_collector_received": True,
            "collector_body_sha256": hashlib.sha256(body).hexdigest(),
            "collector_body_bytes": len(body)}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--broker-report", type=Path, required=True)
    parser.add_argument("--guest-report", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    try:
        payload, proof = build_export(_read(args.broker_report), _read(args.guest_report), time.time_ns())
        transport = export_to_loopback(payload)
    except Exception as exc:
        if os.environ.get("GITHUB_ACTIONS") == "true":
            detail = f"{type(exc).__name__}: {exc}"[:900]
            detail = detail.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
            print(f"::error title=Stage039 OTLP::{detail}", file=sys.stderr)
        raise
    result = {"status": "TWO_VERIFIED_KVM_GUEST_RECEIPTS_EXPORTED_TO_LOOPBACK_OTLP",
              "tenant_traces": proof, **transport, "otlp_protocol": "OTLP/HTTP JSON /v1/traces",
              "guest_application_instrumented": False, "collector_independent_host": False,
              "two_independent_hosts": False, "stage_039_complete": False,
              "production_approved": False}
    with args.report.open("x", encoding="utf-8") as stream:
        json.dump(result, stream, indent=2, sort_keys=True)
        stream.write("\n")
    print(result["status"])


if __name__ == "__main__":
    main()
