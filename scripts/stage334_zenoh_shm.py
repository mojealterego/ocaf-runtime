#!/usr/bin/env python3
"""Real Zenoh 1.10.x interprocess POSIX SHM negotiation and fallback probe.

Test on one Linux kernel. Require received ZBytes.as_shm() to be non-None
when enabled; explicitly verify normal bytes fallback when peer disables SHM.
This is not multi-host zero-copy nor an OCAF production-transport test.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import multiprocessing as mp
import os
import secrets
import socket
import sys
import time

MAX_SECONDS = 35
PAYLOAD_SIZE = 32 * 1024
KEY = "ocaf/stage334/shm/probe"


def config(endpoint: str, receive: bool, shm_enabled: bool):
    import zenoh

    conf = zenoh.Config()
    conf.insert_json5("mode", '"peer"')
    conf.insert_json5("scouting/multicast/enabled", "false")
    conf.insert_json5("scouting/gossip/enabled", "false")
    conf.insert_json5("transport/shared_memory/enabled", json.dumps(shm_enabled))
    conf.insert_json5("transport/shared_memory/mode", '"init"')
    if receive:
        conf.insert_json5("listen/endpoints", json.dumps([endpoint]))
    else:
        conf.insert_json5("connect/endpoints", json.dumps([endpoint]))
    return conf


def subscriber(endpoint, enabled, results, ready, token, expected_hash):
    try:
        import zenoh
        with zenoh.open(config(endpoint, True, enabled)) as session:
            def on_sample(sample):
                try:
                    data = sample.payload.to_bytes()
                    results.put({
                        "kind": "sample", "valid": hashlib.sha256(data).hexdigest() == expected_hash,
                        "shm": sample.payload.as_shm() is not None,
                        "token": data[:16].hex(),
                    })
                except Exception as error:
                    results.put({"kind": "error", "reason": type(error).__name__})

            sub = session.declare_subscriber(KEY, on_sample)
            ready.put("ready")
            deadline = time.monotonic() + MAX_SECONDS
            while time.monotonic() < deadline:
                time.sleep(.1)
            sub.undeclare()
    except Exception as error:
        results.put({"kind": "error", "reason": type(error).__name__})
        ready.put("error")


def publisher(endpoint, token, payload, results):
    try:
        import zenoh
        with zenoh.open(config(endpoint, False, True)) as session:
            provider = zenoh.shm.ShmProvider.default_backend(1024 * 1024)
            pub = session.declare_publisher(KEY)
            for _ in range(18):
                shm = provider.alloc(
                    len(payload), policy=zenoh.shm.BlockOn(zenoh.shm.GarbageCollect())
                )
                shm[:] = payload
                pub.put(shm)
                time.sleep(.25)
            pub.undeclare()
        results.put({"kind": "sent"})
    except Exception as error:
        results.put({"kind": "error", "reason": type(error).__name__})


def run_once(shm_enabled: bool) -> dict:
    import queue
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    endpoint = f"tcp/127.0.0.1:{port}"
    token = secrets.token_bytes(16)
    payload = token + secrets.token_bytes(PAYLOAD_SIZE - 16)
    expected_hash = hashlib.sha256(payload).hexdigest()
    ctx = mp.get_context("spawn")
    results = ctx.Queue()
    ready = ctx.Queue()
    receiver = ctx.Process(
        target=subscriber,
        args=(endpoint, shm_enabled, results, ready, token.hex(), expected_hash),
    )
    sender = ctx.Process(target=publisher, args=(endpoint, token.hex(), payload, results))
    receiver.start()
    try:
        try:
            if ready.get(timeout=15) != "ready":
                raise RuntimeError("Zenoh subscriber initialization failed")
        except queue.Empty as error:
            raise RuntimeError("Zenoh subscriber timed out") from error
        sender.start()
        sender.join(timeout=15)
        if sender.is_alive() or sender.exitcode != 0:
            raise RuntimeError("Zenoh publisher did not finish")
        observed = []
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            try:
                event = results.get(timeout=.4)
            except queue.Empty:
                continue
            if event["kind"] == "error":
                raise RuntimeError("Zenoh runtime rejected probe: " + event["reason"])
            if event["kind"] == "sample":
                observed.append(event)
                if event["valid"] and event["token"] == token.hex() and event["shm"] == shm_enabled:
                    return {
                        "receiver_shm_enabled": shm_enabled,
                        "sample_sha256": expected_hash,
                        "payload_bytes": len(payload),
                        "received_shm": event["shm"],
                        "separate_processes": True,
                    }
        raise RuntimeError(f"Zenoh SHM/fallback validation failed: {observed[:3]}")
    finally:
        if sender.is_alive():
            sender.terminate()
        if receiver.is_alive():
            receiver.terminate()
        sender.join(timeout=5) if sender.pid else None
        receiver.join(timeout=5)


def run() -> dict:
    if sys.platform != "linux" or not os.path.isdir("/dev/shm"):
        raise RuntimeError("POSIX shared memory requires a Linux /dev/shm host")
    import zenoh

    if not hasattr(zenoh, "shm") or not hasattr(zenoh.shm, "ShmProvider"):
        raise RuntimeError("Installed Zenoh Python binary has no shared-memory support")
    with_shm = run_once(True)
    without_shm = run_once(False)
    return {
        "status": "ZENOH_SAME_HOST_TWO_PROCESS_SHM_AND_FALLBACK_VERIFIED",
        "zenoh_version": getattr(zenoh, "__version__", "library_version_not_exposed"),
        "same_host_only": True,
        "cross_host_zero_copy_verified": False,
        "production_omat_ingress_tested": False,
        "shm_delivery": with_shm,
        "fallback_delivery": without_shm,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report")
    args = parser.parse_args()
    try:
        result = run()
        encoded = json.dumps(result, sort_keys=True) + "\n"
        if args.report:
            with open(args.report, "w", encoding="utf-8") as file:
                file.write(encoded)
        print(encoded, end="")
        return 0
    except Exception as error:
        print(f"ZENOH_SHM_FAIL: {type(error).__name__}: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
