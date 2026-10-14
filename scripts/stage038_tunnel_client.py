#!/usr/bin/env python3
"""GitHub OIDC-authenticated byte tunnel. OCAF TLS is end-to-end through the relay."""
import argparse
import asyncio
import os
from pathlib import Path
import sys
from urllib.parse import urlencode
import aiohttp

AUDIENCE = "ocaf-038-twohost-relay"


async def identity(session):
    url = os.getenv("ACTIONS_ID_TOKEN_REQUEST_URL", "")
    secret = os.getenv("ACTIONS_ID_TOKEN_REQUEST_TOKEN", "")
    if not url or not secret or os.getenv("GITHUB_ACTIONS") != "true":
        raise RuntimeError("GitHub OIDC identity unavailable")
    sep = "&" if "?" in url else "?"
    async with session.get(url + sep + urlencode({"audience": AUDIENCE}),
                           headers={"Authorization": "bearer " + secret},
                           timeout=aiohttp.ClientTimeout(total=20)) as response:
        response.raise_for_status()
        jwt = (await response.json())["value"]
    if not isinstance(jwt, str) or not jwt:
        raise RuntimeError("OIDC JWT missing")
    return jwt


async def exchange(base, run_id, item, method, path):
    if not path.is_absolute():
        raise ValueError("require absolute private file path")
    async with aiohttp.ClientSession() as session:
        headers = {"Authorization": "Bearer " + await identity(session)}
        url = base + "/exchange/" + run_id + "/" + item
        if method == "put":
            data = path.read_bytes()
            if not 0 < len(data) <= 16384:
                raise ValueError("invalid exchange payload size")
            async with session.post(url, data=data, headers=headers,
                                    timeout=aiohttp.ClientTimeout(total=25)) as response:
                response.raise_for_status()
        else:
            data = None
            for i in range(240):
                # Rotate short-lived OIDC tokens while waiting for the other VM.
                if i and i % 15 == 0:
                    headers["Authorization"] = "Bearer " + await identity(session)
                async with session.get(url, headers=headers,
                                       timeout=aiohttp.ClientTimeout(total=25)) as response:
                    if response.status == 404:
                        await asyncio.sleep(2)
                        continue
                    response.raise_for_status()
                    data = await response.read()
                    break
            if data is None or not 0 < len(data) <= 16384:
                raise TimeoutError("exchange item not ready")
            with path.open("xb") as output:
                output.write(data)
            path.chmod(0o600)
    print("OCAF038_OIDC_PUBLIC_EXCHANGE_" + method.upper() + "_" + item, flush=True)


async def bridge(ws, reader, writer):
    async def tcp_to_ws():
        while True:
            chunk = await reader.read(65536)
            if not chunk:
                break
            await ws.send_bytes(chunk)
    async def ws_to_tcp():
        async for event in ws:
            if event.type == aiohttp.WSMsgType.BINARY:
                writer.write(event.data)
                await writer.drain()
            elif event.type == aiohttp.WSMsgType.TEXT:
                raise RuntimeError("unexpected plaintext in TLS data tunnel")
            else:
                break
    tasks = [asyncio.create_task(tcp_to_ws()), asyncio.create_task(ws_to_tcp())]
    try:
        finished, _ = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
        for task in finished:
            task.result()
    finally:
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)
        writer.close()
        await writer.wait_closed()
        await ws.close()


async def connection(session, base, run_id, role, local=None, reader=None, writer=None):
    token = await identity(session)
    url = base.replace("https://", "wss://", 1) + "/tunnel/" + run_id + "/" + role
    async with session.ws_connect(url, headers={"Authorization": "Bearer " + token},
                                  max_msg_size=262144, timeout=25) as ws:
        while True:
            hello = await ws.receive(timeout=220)
            if hello.type != aiohttp.WSMsgType.TEXT or hello.data not in ("QUEUED", "PAIRED"):
                raise RuntimeError("invalid relay pairing")
            if hello.data == "PAIRED":
                break
        if local is not None:
            reader, writer = await asyncio.open_connection(*local)
        if reader is None or writer is None:
            raise RuntimeError("missing TCP endpoint")
        await bridge(ws, reader, writer)


async def upstream_worker(base, run_id, port):
    async with aiohttp.ClientSession() as session:
        while True:
            try:
                await connection(session, base, run_id, "upstream", local=("127.0.0.1", port))
            except (aiohttp.ClientError, OSError, TimeoutError, RuntimeError, asyncio.TimeoutError):
                print("OCAF038_RELAY_UPSTREAM_RECONNECT", flush=True)
                await asyncio.sleep(2)


async def downstream_listener(base, run_id, port):
    async def handler(reader, writer):
        async with aiohttp.ClientSession() as session:
            try:
                await connection(session, base, run_id, "downstream", reader=reader, writer=writer)
            except (aiohttp.ClientError, OSError, TimeoutError, RuntimeError, asyncio.TimeoutError):
                writer.close()
                await writer.wait_closed()
                print("OCAF038_RELAY_DOWNSTREAM_FAILED", flush=True)
    server = await asyncio.start_server(handler, "127.0.0.1", port)
    print("OCAF038_DOWNSTREAM_LOCAL_TCP_LISTENING", flush=True)
    async with server:
        await server.serve_forever()


async def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--relay", default="https://ocaf038-free-oidc-relay.onrender.com")
    ap.add_argument("--run-id", default=os.getenv("GITHUB_RUN_ID", ""))
    ap.add_argument("--mode", choices=("upstream", "downstream", "put", "get"), required=True)
    ap.add_argument("--item", default="")
    ap.add_argument("--file", type=Path)
    ap.add_argument("--port", type=int, default=0)
    ap.add_argument("--workers", type=int, default=8)
    args = ap.parse_args()
    if not args.relay.startswith("https://") or "@" in args.relay or "#" in args.relay:
        ap.error("HTTPS-only relay")
    if not args.run_id.isdigit():
        ap.error("GITHUB_RUN_ID required")
    if args.mode in ("put", "get"):
        if args.file is None or args.item not in (
            "ca.pem", "agent-a.csr", "agent-b.csr", "transport.pub",
            "agent-a.pem", "agent-b.pem", "hmac.sealed", "result.json"
        ):
            ap.error("known item and --file required")
        await exchange(args.relay.rstrip("/"), args.run_id, args.item, args.mode, args.file)
    elif args.mode == "downstream":
        if args.port < 1:
            ap.error("--port required")
        await downstream_listener(args.relay.rstrip("/"), args.run_id, args.port)
    else:
        if args.port < 1 or not 1 <= args.workers <= 12:
            ap.error("1..12 workers and --port required")
        await asyncio.gather(*(upstream_worker(args.relay.rstrip("/"), args.run_id, args.port)
                               for _ in range(args.workers)))


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except (ValueError, RuntimeError, aiohttp.ClientError, OSError, TimeoutError) as err:
        print("OCAF038_CI_TUNNEL_ERROR=" + type(err).__name__, file=sys.stderr)
        sys.exit(2)
