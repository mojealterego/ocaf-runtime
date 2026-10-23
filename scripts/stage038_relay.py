#!/usr/bin/env python3
"""Zero-cost, OIDC-gated websocket byte tunnel for two ephemeral GitHub VMs.
Only forwards opaque TLS records; no secrets or production authorizations.
"""
import asyncio
from collections import defaultdict
import os
import re
import time

from aiohttp import web, WSMsgType
import jwt
from jwt import PyJWKClient

AUDIENCE = "ocaf-038-twohost-relay"
ISSUER = "https://token.actions.githubusercontent.com"
WORKFLOW = "mojealterego/ocaf-runtime/.github/workflows/stage038-twohost.yml@refs/heads/main"
JWK = PyJWKClient(ISSUER + "/.well-known/jwks", cache_jwk_set=True, lifespan=900, timeout=8)
ITEMS = frozenset({"ca.pem", "agent-a.csr", "agent-b.csr", "transport.pub",
                   "agent-a.pem", "agent-b.pem", "hmac.sealed", "result.json"})
PENDING = defaultdict(asyncio.Queue)
MAILBOX = {}


def verify_oidc(token, run_id):
    if not token or len(token) > 16000 or not re.fullmatch(r"[1-9][0-9]{0,19}", run_id):
        raise web.HTTPUnauthorized(text="invalid credential")
    try:
        claims = jwt.decode(token, JWK.get_signing_key_from_jwt(token).key,
                            algorithms=["RS256"], audience=AUDIENCE, issuer=ISSUER,
                            options={"require": ["iss", "aud", "exp", "iat", "sub",
                                                 "repository", "run_id", "ref",
                                                 "workflow_ref", "event_name", "sha"]})
    except (jwt.PyJWTError, OSError, ValueError) as exc:
        raise web.HTTPUnauthorized(text="identity could not be verified") from exc
    if not (claims.get("repository") == "mojealterego/ocaf-runtime"
            and claims.get("run_id") == run_id
            and claims.get("ref") == "refs/heads/main"
            and claims.get("workflow_ref") == WORKFLOW
            and claims.get("event_name") in ("push", "workflow_dispatch")
            and re.fullmatch("[0-9a-f]{40}", str(claims.get("sha", "")))):
        raise web.HTTPForbidden(text="workflow identity not authorized")
    return claims


def authenticated_run(request):
    run_id = request.match_info["run_id"]
    auth = request.headers.get("Authorization", "")
    if not auth.startswith("Bearer ") or auth.count(" ") != 1:
        raise web.HTTPUnauthorized(text="OIDC required")
    verify_oidc(auth[7:], run_id)
    return run_id


async def health(_request):
    return web.json_response({"status": "READY", "purpose": "CI_EPHEMERAL_ONLY",
                              "production_approved": False})


async def exchange(request):
    run_id = authenticated_run(request)
    item = request.match_info["item"]
    if item not in ITEMS:
        raise web.HTTPNotFound()
    key = (run_id, item)
    if request.method == "POST":
        if request.content_length is None or request.content_length > 16384:
            raise web.HTTPRequestEntityTooLarge(max_size=16384,
                                                 actual_size=request.content_length or 0)
        data = await request.read()
        if not 0 < len(data) <= 16384:
            raise web.HTTPBadRequest(text="invalid exchange payload")
        existing = MAILBOX.get(key)
        if existing is not None:
            # Retry after a lost HTTP response: preserve the original timestamp.
            if existing[0] == data:
                return web.Response(status=200, text="already present")
            raise web.HTTPConflict(text="exchange item already exists")
        if len(MAILBOX) >= 64:
            raise web.HTTPServiceUnavailable(text="mailbox limit")
        MAILBOX[key] = (data, time.monotonic())
        return web.Response(status=201, text="created")
    entry = MAILBOX.get(key)
    if entry is None:
        raise web.HTTPNotFound(text="exchange item not ready")
    return web.Response(body=entry[0], content_type="application/octet-stream")


async def bridge(a, b):
    async def transfer(source, destination):
        async for msg in source:
            if msg.type == WSMsgType.BINARY:
                await destination.send_bytes(msg.data)
            elif msg.type == WSMsgType.TEXT:
                raise ValueError("data tunnel permits binary only")
            else:
                break
    tasks = [asyncio.create_task(transfer(a, b)),
             asyncio.create_task(transfer(b, a))]
    try:
        finished, _ = await asyncio.wait(tasks, timeout=160,
                                         return_when=asyncio.FIRST_COMPLETED)
        for task in finished:
            task.result()
    finally:
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)
        await asyncio.gather(a.close(), b.close(), return_exceptions=True)


async def tunnel(request):
    run_id = authenticated_run(request)
    role = request.match_info["role"]
    if role not in ("upstream", "downstream"):
        raise web.HTTPNotFound()
    queue = PENDING[run_id]
    if len(PENDING) > 32 or queue.qsize() >= 12:
        raise web.HTTPServiceUnavailable(text="CI relay capacity exhausted")
    ws = web.WebSocketResponse(max_msg_size=262144)
    await ws.prepare(request)
    if role == "upstream":
        completed = asyncio.get_running_loop().create_future()
        await queue.put((ws, completed))
        await ws.send_str("QUEUED")
        try:
            await asyncio.wait_for(completed, timeout=270)
        except asyncio.TimeoutError:
            pass
        finally:
            await ws.close()
        return ws
    try:
        end_time = asyncio.get_running_loop().time() + 90
        while True:
            remaining = end_time - asyncio.get_running_loop().time()
            if remaining <= 0:
                raise asyncio.TimeoutError()
            upstream, completed = await asyncio.wait_for(queue.get(), timeout=remaining)
            if not upstream.closed and not completed.done():
                break
    except asyncio.TimeoutError:
        await ws.close(message=b"no upstream")
        return ws
    await upstream.send_str("PAIRED")
    await ws.send_str("PAIRED")
    try:
        await bridge(upstream, ws)
    finally:
        if not completed.done():
            completed.set_result(None)
    return ws


async def reap(app):
    while True:
        await asyncio.sleep(60)
        for key, (_, inserted) in list(MAILBOX.items()):
            if inserted < time.monotonic() - 900:
                del MAILBOX[key]


async def lifespan(app):
    task = asyncio.create_task(reap(app))
    yield
    task.cancel()
    await asyncio.gather(task, return_exceptions=True)


def create_app():
    app = web.Application(client_max_size=16384)
    app.router.add_get("/health", health)
    app.router.add_route("GET", "/exchange/{run_id}/{item}", exchange)
    app.router.add_route("POST", "/exchange/{run_id}/{item}", exchange)
    app.router.add_get("/tunnel/{run_id}/{role}", tunnel)
    app.cleanup_ctx.append(lifespan)
    return app


if __name__ == "__main__":
    web.run_app(create_app(), host="0.0.0.0", port=int(os.getenv("PORT", "10000")),
                access_log=None)
