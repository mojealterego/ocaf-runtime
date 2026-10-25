#!/usr/bin/env python3
"""Actual host-A kernel packet partition of a host-B mTLS fencing authority.

Run as root on A. B must already run stage039_mtls_fencing.py serve with a
private SQLite ledger and separately provisioned TLSv1.3 CA/server cert.
This proves interhost transport denial and recovery, not HA or consensus.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import ipaddress
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import time

import stage039_mtls_fencing as fence


class PartitionDenied(ValueError):
    pass


def run_command(argv: list[str]) -> None:
    result = subprocess.run(argv, capture_output=True, text=True, timeout=5, check=False)
    if result.returncode:
        raise PartitionDenied(f"kernel firewall command failed: {argv[1]}: {result.stderr[-300:]}")


def check_response(response: dict, status: str, epoch: int) -> None:
    if (not isinstance(response, dict) or response.get("status") != status
            or response.get("tenant_id") != fence.TENANT
            or response.get("epoch") != epoch
            or response.get("production_approved") is not False):
        raise PartitionDenied("unexpected mTLS fencing response")


def drop_counter(raw: str, address: str, port: int) -> int:
    """Require one exact scoped nft rule and its observed kernel packet count."""
    try:
        document = json.loads(raw)
        rules = [item["rule"] for item in document["nftables"] if "rule" in item]
    except (KeyError, TypeError, ValueError) as exc:
        raise PartitionDenied("kernel nftables JSON counter missing") from exc
    if len(rules) != 1 or not isinstance(rules[0].get("expr"), list):
        raise PartitionDenied("expected exactly one scoped kernel drop rule")
    expressions = rules[0]["expr"]
    def match(protocol: str, field: str, expected: str | int) -> bool:
        return any(isinstance(expr, dict) and expr.get("match") == {
            "op": "==", "left": {"payload": {"protocol": protocol, "field": field}},
            "right": expected} for expr in expressions)
    if (not match("ip", "daddr", address) or not match("tcp", "dport", port)
            or not any(isinstance(expr, dict) and "drop" in expr for expr in expressions)):
        raise PartitionDenied("kernel packet drop rule differs from exact authority endpoint")
    counts = [expr["counter"].get("packets") for expr in expressions
              if isinstance(expr, dict) and isinstance(expr.get("counter"), dict)]
    if len(counts) != 1 or type(counts[0]) is not int or counts[0] < 0:
        raise PartitionDenied("kernel packet counter missing from scoped rule")
    return counts[0]


def drill(host: str, port: int, cert: Path, key: Path, ca: Path) -> dict:
    if os.geteuid() != 0 or shutil.which("nft") is None:
        raise PartitionDenied("root Linux nftables execution required")
    address = ipaddress.ip_address(host)
    if address.version != 4 or address.is_loopback or address.is_unspecified:
        raise PartitionDenied("host-B IPv4 address required")
    if not 1024 <= port <= 65535:
        raise PartitionDenied("bounded mTLS port required")
    route = subprocess.run(["ip", "-j", "route", "get", str(address)],
                           capture_output=True, text=True, timeout=5, check=True)
    paths = json.loads(route.stdout)
    if (not isinstance(paths, list) or len(paths) != 1 or
            paths[0].get("type") == "local" or paths[0].get("dev") == "lo"):
        raise PartitionDenied("authority address routes to local host or lacks remote route")
    table = "ocaf039_" + secrets.token_hex(4)
    request_id = secrets.token_hex(16)
    epoch = time.time_ns()
    def call(action: str):
        return fence.client_rpc(host=host, port=port, cert=cert, key=key, ca=ca,
            request={"version": 1, "action": action, "tenant_id": fence.TENANT,
                     "lease_id": request_id, "epoch": epoch, "ttl_seconds": 20})
    claimed = call("claim")
    check_response(claimed, "LEASE_GRANTED", epoch)
    table_created = False
    try:
        run_command(["nft", "add", "table", "ip", table])
        table_created = True
        run_command(["nft", "add", "chain", "ip", table, "output",
                     "{ type filter hook output priority -100 ; policy accept ; }"])
        run_command(["nft", "add", "rule", "ip", table, "output", "ip", "daddr",
                     str(address), "tcp", "dport", str(port), "counter", "drop"])
        def observed_packets():
            listing = subprocess.run(["nft", "-j", "list", "chain", "ip", table, "output"],
                                     capture_output=True, text=True, timeout=5, check=True).stdout
            return drop_counter(listing, str(address), port)
        initial_packets = observed_packets()
        try:
            response = call("renew")
        except (OSError, ValueError):
            pass
        else:
            raise PartitionDenied("mTLS lease renewed during actual packet partition: " + repr(response))
        packets_after_blocked_renewal = observed_packets()
        if packets_after_blocked_renewal <= initial_packets:
            raise PartitionDenied("failed renewal did not hit scoped kernel packet drop rule")
        if fence.safe_to_write(False, int(time.time()), claimed["expires_unix_secs"]):
            raise PartitionDenied("client accepted write after losing authority")
    finally:
        if table_created:
            run_command(["nft", "delete", "table", "ip", table])
    restored = call("renew")
    check_response(restored, "LEASE_RENEWED", epoch)
    released = call("release")
    check_response(released, "LEASE_RELEASED", epoch)
    return {"schema_version": 1,
            "status": "REAL_INTERHOST_TLS13_PARTITION_FENCING_RECOVERY_OBSERVED",
            "observed_at_utc": datetime.now(timezone.utc).isoformat(),
            "client_kernel_boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "authority_ipv4": str(address), "authority_port": port,
            "nft_output_packet_drop_observed": True,
            "nft_drop_packets_before_renewal": initial_packets,
            "nft_drop_packets_after_blocked_renewal": packets_after_blocked_renewal,
            "renewal_during_packet_partition_denied": True,
            "write_after_lost_authority_denied": True,
            "renewal_after_packet_recovery_proved": True,
            "lease_released_after_recovery": True,
            "remote_authority_host_identity_attested": False,
            "multi_host_failover_tested": False,
            "distributed_quorum_verified": False,
            "stage_039_complete": False, "production_approved": False}


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--host-b-ipv4", required=True)
    p.add_argument("--port", type=int, default=24399)
    for name in ("cert", "key", "ca", "report"):
        p.add_argument("--" + name, required=True, type=Path)
    p.add_argument("--ack-real-host-network-partition", action="store_true")
    x = p.parse_args()
    if not x.ack_real_host_network_partition:
        p.error("explicit interhost nftables partition acknowledgment required")
    try:
        if x.report.exists():
            raise PartitionDenied("report path already exists")
        result = drill(x.host_b_ipv4, x.port, x.cert, x.key, x.ca)
        x.report.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n")
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        p.error(str(exc))
    print(result["status"])


if __name__ == "__main__":
    main()
