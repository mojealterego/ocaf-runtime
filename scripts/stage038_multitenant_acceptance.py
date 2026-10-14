#!/usr/bin/env python3
"""Real systemd two-tenant CI acceptance and controlled TCP / SIGKILL faults.

This is evidence from a disposable GitHub runner, never production acceptance.
All identities, modules and keys must be generated for this run only.
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import select
import socket
import socketserver
import subprocess
import sys
import threading
import time

MAX_CLIENT_TIME = 14


class FaultServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True

    def __init__(self, destination: tuple[str, int]):
        super().__init__(("127.0.0.1", 0), FaultHandler)
        self.destination = destination
        self.blocked = threading.Event()


class FaultHandler(socketserver.BaseRequestHandler):
    def handle(self) -> None:
        if self.server.blocked.is_set():
            return
        try:
            with socket.create_connection(self.server.destination, timeout=3) as upstream:
                self.request.settimeout(4)
                upstream.settimeout(4)
                while not self.server.blocked.is_set():
                    ready, _, _ = select.select([self.request, upstream], [], [], 1)
                    for source in ready:
                        data = source.recv(65536)
                        if not data:
                            return
                        (upstream if source is self.request else self.request).sendall(data)
        except (OSError, ValueError):
            return


def invoke(binary: Path, root: Path, tenant: str, principal: str,
           port: int, action: str = "call", tool: str = "") -> subprocess.CompletedProcess:
    data = root / tenant
    env = os.environ.copy()
    env.update({
        "OCAF_ROLE": "client", "OCAF_NODE_ID": principal,
        "OCAF_PEER_ID": f"coord-{tenant}", "OCAF_RPC_INGRESS": "tls-bound",
        "OCAF_ACTION": action, "OCAF_TOOL": tool, "OCAF_MESSAGE": "synthetic_staging_only",
        "OCAF_TLS_RPC_ADDRESS": f"127.0.0.1:{port}",
        "OCAF_TLS_RPC_SERVER_DNS": "localhost",
        "OCAF_TLS_SERVER_CA_FILE": str(data / "ca.pem"),
        "OCAF_TLS_CLIENT_CERT_FILE": str(data / f"{principal}.pem"),
        "OCAF_TLS_CLIENT_KEY_FILE": str(data / f"{principal}.key"),
        "OCAF_RPC_CERT_KEY_FILE": str(data / f"{principal}.key"),
    })
    peers = json.loads((data / "peers.json").read_text(encoding="utf-8"))
    env["OCAF_SHARED_KEY_B64"] = peers[principal]["key_b64"]
    return subprocess.run([str(binary)], env=env, capture_output=True, timeout=MAX_CLIENT_TIME,
                          check=False)


def error_category(stderr: bytes) -> str:
    """Print fixed categories only; raw credential-bearing diagnostics stay private."""
    for label, signature in (
        ("service_not_listening", b"Connection refused"),
        ("tls_identity_rejected", b"certificate"),
        ("rpc_policy_rejected", b"remote RPC rejected"),
        ("rpc_execution_failed", b"remote tool call failed"),
        ("transport_closed", b"early eof"),
        ("transport_timeout", b"deadline has elapsed"),
    ):
        if signature.lower() in stderr.lower():
            return label
    return "unclassified_client_failure"


def require_positive(binary: Path, root: Path, tenant: str, principal: str,
                     tool: str, port: int) -> None:
    result = invoke(binary, root, tenant, principal, port, tool=tool)
    if result.returncode != 0:
        raise RuntimeError(f"authenticated tenant {tenant} execution failed: "
                           f"{error_category(result.stderr)} (exit={result.returncode})")
    try:
        reply = json.loads(result.stdout)
    except ValueError as exc:
        raise RuntimeError("non-JSON RPC response") from exc
    if (reply.get("status") != "ok"
            or reply.get("sender") != f"coord-{tenant}"
            or reply.get("recipient") != principal
            or reply.get("output") != {"ok": True, "tenant": tenant}):
        raise RuntimeError(f"tenant {tenant} response/principal isolation broken")


def require_policy_deny(binary: Path, root: Path, tenant: str, wrong_principal: str,
                        tool: str, port: int) -> None:
    result = invoke(binary, root, tenant, wrong_principal, port, tool=tool)
    if result.returncode == 0 or result.stdout or result.stderr != b"Error: remote RPC rejected\n":
        raise RuntimeError(f"missing policy-layer deny for {wrong_principal} against {tenant}")


def require_untrusted_ca_deny(binary: Path, root: Path, tenant: str, cert_tenant: str,
                              principal: str, tool: str, port: int) -> None:
    # An entirely different tenant trust root is pinned on the *client*.
    # Rejection must not be accepted merely because the service went offline.
    result = invoke(binary, root, cert_tenant, principal, port, tool=tool)
    if result.returncode == 0 or result.stdout:
        raise RuntimeError("cross-tenant trust-root request unexpectedly succeeded")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--port-a", type=int, required=True)
    parser.add_argument("--port-b", type=int, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args(argv)
    if os.environ.get("GITHUB_ACTIONS") != "true" or not all(
            0 < p < 65536 for p in (args.port_a, args.port_b)):
        print("NO_GO: requires isolated GitHub Linux runner and valid ports", file=sys.stderr)
        return 2
    root, binary = args.directory.resolve(), args.binary.resolve()
    checks: list[str] = []
    before, after = None, None
    proxy = FaultServer(("127.0.0.1", args.port_a))
    thread = threading.Thread(target=proxy.serve_forever, daemon=True)
    thread.start()
    try:
        a, b = "tenant-a", "tenant-b"
        atool, btool = "ocaf_ci_tenant_a", "ocaf_ci_tenant_b"
        # systemd-run --service-type=exec proves exec, not that TLS has bound.
        # Wait for real *signed RPC execution* on both listeners before chaos.
        for tenant, principal, tool, port in ((a, "agent-a", atool, args.port_a),
                                               (b, "agent-b", btool, args.port_b)):
            for attempt in range(25):
                try:
                    require_positive(binary, root, tenant, principal, tool, port)
                    break
                except RuntimeError:
                    if attempt == 24:
                        raise
                    time.sleep(.3)
        checks.append("two_independent_signed_tenants_success")
        with ThreadPoolExecutor(max_workers=8) as pool:
            tasks = [pool.submit(require_positive, binary, root, a if i % 2 == 0 else b,
                                 "agent-a" if i % 2 == 0 else "agent-b",
                                 atool if i % 2 == 0 else btool,
                                 args.port_a if i % 2 == 0 else args.port_b)
                     for i in range(12)]
            for task in tasks:
                task.result()
        checks.append("simultaneous_tenant_a_and_b_rpc_execution")
        require_policy_deny(binary, root, a, "agent-b", atool, args.port_a)
        require_policy_deny(binary, root, b, "agent-a", btool, args.port_b)
        checks.append("cross_principal_policy_denial_on_each_tenant")
        require_untrusted_ca_deny(binary, root, a, b, "agent-b", btool, args.port_a)
        require_untrusted_ca_deny(binary, root, b, a, "agent-a", atool, args.port_b)
        require_positive(binary, root, a, "agent-a", atool, args.port_a)
        require_positive(binary, root, b, "agent-b", btool, args.port_b)
        checks.append("independent_ca_rejection_followed_by_live_positive_controls")
        require_positive(binary, root, a, "agent-a", atool, proxy.server_address[1])
        proxy.blocked.set()
        failure = invoke(binary, root, a, "agent-a", proxy.server_address[1], tool=atool)
        if failure.returncode == 0:
            raise RuntimeError("partition proxy failed to block tenant A")
        require_positive(binary, root, b, "agent-b", btool, args.port_b)
        proxy.blocked.clear()
        require_positive(binary, root, a, "agent-a", atool, proxy.server_address[1])
        checks.append("tenant_a_tcp_fault_tenant_b_continues_then_recovery")
        command = ["systemctl", "show", "-p", "MainPID", "--value", "ocaf-tenant-a.service"]
        before = int(subprocess.check_output(command, timeout=10, text=True).strip())
        if before <= 1:
            raise RuntimeError("no active tenant A main PID before SIGKILL")
        subprocess.run(["sudo", "systemctl", "kill", "--kill-who=main", "--signal=SIGKILL",
                        "ocaf-tenant-a.service"], check=True, capture_output=True, timeout=10)
        require_positive(binary, root, b, "agent-b", btool, args.port_b)
        for _ in range(50):
            time.sleep(.3)
            result = subprocess.run(command, capture_output=True, timeout=5, text=True, check=False)
            if result.returncode == 0:
                candidate = int(result.stdout.strip() or "0")
                if candidate > 1 and candidate != before:
                    try:
                        require_positive(binary, root, a, "agent-a", atool, args.port_a)
                        after = candidate
                        break
                    except RuntimeError:
                        pass
        if after is None:
            raise RuntimeError("tenant A did not recover after enforced SIGKILL")
        require_positive(binary, root, b, "agent-b", btool, args.port_b)
        checks.append("main_pid_sigkill_systemd_restart_no_tenant_b_interruption")
        if args.report.exists():
            raise RuntimeError("cannot overwrite evidence")
        report = {
            "schema_version": 1, "git_commit": os.environ["GITHUB_SHA"],
            "github_run_id": os.environ["GITHUB_RUN_ID"],
            "kernel_boot_id_sha256": hashlib.sha256(
                Path("/proc/sys/kernel/random/boot_id").read_bytes().strip()).hexdigest(),
            "status": "TWO_TENANT_LINUX_SYSTEMD_CI_PASS_NOT_PRODUCTION_GO",
            "tenants": ["tenant-a", "tenant-b"], "independent_certificate_authorities": True,
            "systemd_tenant_a_pid_before": before, "systemd_tenant_a_pid_after": after,
            "checks": checks, "ephemeral_ci": True,
            "production_approved": False, "independent_audit": False,
            "operator_signed": False, "real_inter_host_network_partition": False,
        }
        fd = os.open(args.report, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
        with os.fdopen(fd, "w", encoding="utf-8") as out:
            json.dump(report, out, indent=2, sort_keys=True)
            out.write("\n")
        print(json.dumps({"status": report["status"], "checks": len(checks),
                          "production_approved": False}))
        return 0
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as exc:
        print(f"NO_GO: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2
    finally:
        proxy.shutdown()
        proxy.server_close()
        thread.join(timeout=3)


if __name__ == "__main__":
    raise SystemExit(main())
