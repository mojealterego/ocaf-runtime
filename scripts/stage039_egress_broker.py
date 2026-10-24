#!/usr/bin/env python3
"""Bounded HTTPS egress through a signed, tenant-bound destination permit.

The caller identity must come from an authenticated broker transport. This
module neither creates that transport nor grants guest network access. Its
replay set is process-local, so durable broker replay protection is still
required. The default destination policy admits only globally routable IPs.
"""
from __future__ import annotations

import ipaddress
import re
import socket
import ssl
import threading
import time
from typing import Callable, Mapping
from urllib.parse import urlsplit

import stage039_signed_manifest

HOST = re.compile(r"(?=.{1,253}\Z)[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?\Z")
TENANT = re.compile(r"[a-z][a-z0-9_-]{0,62}\Z")
NONCE = re.compile(r"[0-9a-f]{32}\Z")
MAX_HEADER = 8192


class EgressDenied(ValueError):
    pass


class EgressBroker:
    def __init__(self, *, trusted_keys: Mapping[str, bytes],
                 ssl_context: ssl.SSLContext | None = None,
                 allowed_ports: frozenset[int] = frozenset({443}),
                 allowed_networks: tuple[ipaddress._BaseNetwork, ...] = (),
                 resolver: Callable = socket.getaddrinfo,
                 clock: Callable[[], float] = time.time):
        if not trusted_keys or not isinstance(allowed_ports, frozenset) or not allowed_ports:
            raise EgressDenied("trusted signer and bounded ports required")
        if any(type(port) is not int or not 1 <= port <= 65535 for port in allowed_ports):
            raise EgressDenied("invalid permitted port")
        self.trusted_keys = dict(trusted_keys)
        self.ssl_context = ssl_context or ssl.create_default_context()
        if self.ssl_context.verify_mode != ssl.CERT_REQUIRED or not self.ssl_context.check_hostname:
            raise EgressDenied("TLS peer and hostname verification required")
        self.allowed_ports = allowed_ports
        self.allowed_networks = tuple(allowed_networks)
        self.resolver = resolver
        self.clock = clock
        self._used: set[tuple[str, str]] = set()
        self._lock = threading.Lock()

    def _permit(self, envelope: bytes, tenant_id: str) -> tuple[str, str, int, str, int]:
        if not isinstance(tenant_id, str) or TENANT.fullmatch(tenant_id) is None:
            raise EgressDenied("authenticated tenant identity required")
        try:
            payload = stage039_signed_manifest.verify(envelope, self.trusted_keys)["payload"]
        except stage039_signed_manifest.SignedManifestError as exc:
            raise EgressDenied("untrusted destination permit") from exc
        now = self.clock()
        if (not isinstance(payload, dict) or set(payload) !=
            {"schema_version", "tenant_id", "nonce", "url", "expires_unix", "max_bytes"}
            or payload["tenant_id"] != tenant_id
            or not isinstance(payload["nonce"], str)
            or NONCE.fullmatch(payload["nonce"]) is None
            or type(payload["expires_unix"]) is not int
            or not now < payload["expires_unix"] <= now + 300
            or type(payload["max_bytes"]) is not int
            or not 1 <= payload["max_bytes"] <= 1024 * 1024
            or not isinstance(payload["url"], str) or len(payload["url"]) > 2048):
            raise EgressDenied("permit scope, expiry or response bound invalid")
        try:
            url = urlsplit(payload["url"])
            port = 443 if url.port is None else url.port
            host = url.hostname
        except ValueError as exc:
            raise EgressDenied("malformed destination URL") from exc
        if (url.scheme != "https" or not host or host != host.lower()
            or HOST.fullmatch(host) is None or ".." in host
            or url.username is not None or url.password is not None
            or url.fragment or port not in self.allowed_ports
            or any(ord(ch) < 0x21 or ord(ch) > 0x7e for ch in payload["url"])):
            raise EgressDenied("destination URL outside signed HTTPS policy")
        try:
            ipaddress.ip_address(host)
        except ValueError:
            pass
        else:
            raise EgressDenied("IP literal destinations are prohibited")
        path = url.path or "/"
        if url.query:
            path += "?" + url.query
        if "#" in path or "\r" in path or "\n" in path:
            raise EgressDenied("unsafe request target")
        with self._lock:
            key = (tenant_id, payload["nonce"])
            if key in self._used:
                raise EgressDenied("replayed destination permit")
            self._used.add(key)
        return host, path, port, payload["nonce"], payload["max_bytes"]

    def _address(self, host: str, port: int) -> tuple[int, tuple, ipaddress._BaseAddress]:
        try:
            records = self.resolver(host, port, type=socket.SOCK_STREAM, proto=socket.IPPROTO_TCP)
        except (OSError, ValueError) as exc:
            raise EgressDenied("destination DNS unavailable") from exc
        if not records or len(records) > 32:
            raise EgressDenied("missing or excessive DNS addresses")
        selected = []
        for family, kind, proto, _, sockaddr in records:
            if family not in (socket.AF_INET, socket.AF_INET6) or kind != socket.SOCK_STREAM:
                raise EgressDenied("unexpected DNS address family")
            try:
                ip = ipaddress.ip_address(sockaddr[0])
            except (ValueError, IndexError, TypeError) as exc:
                raise EgressDenied("malformed DNS address") from exc
            if (not ip.is_global and not any(ip in network for network in self.allowed_networks)):
                raise EgressDenied("private, metadata or reserved DNS address denied")
            if len(sockaddr) < 2 or sockaddr[1] != port:
                raise EgressDenied("DNS answer changed signed destination port")
            selected.append((family, sockaddr, ip))
        return selected[0]

    def fetch(self, *, signed_permit: bytes, authenticated_tenant: str) -> bytes:
        host, path, port, _, limit = self._permit(signed_permit, authenticated_tenant)
        family, sockaddr, ip = self._address(host, port)
        raw = socket.socket(family, socket.SOCK_STREAM)
        deadline = time.monotonic() + 10
        raw.settimeout(5)
        try:
            raw.connect(sockaddr)  # Literal IP chosen from the single policy-checked DNS lookup.
            if (ipaddress.ip_address(raw.getpeername()[0]) != ip
                or raw.getpeername()[1] != port):
                raise EgressDenied("connected peer differs from pinned DNS answer")
            raw.settimeout(max(0.1, min(5, deadline - time.monotonic())))
            with self.ssl_context.wrap_socket(raw, server_hostname=host) as tls:
                raw = None
                def recv_bound(length: int) -> bytes:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        raise EgressDenied("upstream response deadline expired")
                    tls.settimeout(min(5, remaining))
                    return tls.recv(length)
                authority = host if port == 443 else f"{host}:{port}"
                request = (f"GET {path} HTTP/1.1\r\nHost: {authority}\r\n"
                           "Accept: */*\r\nConnection: close\r\n\r\n").encode("ascii")
                tls.sendall(request)
                data = bytearray()
                while b"\r\n\r\n" not in data:
                    chunk = recv_bound(4096)
                    if not chunk or len(data) + len(chunk) > MAX_HEADER + limit:
                        raise EgressDenied("upstream response header/body exceeded bound")
                    data.extend(chunk)
                    if b"\r\n\r\n" not in data and len(data) > MAX_HEADER:
                        raise EgressDenied("upstream response header exceeded bound")
                header, body = bytes(data).split(b"\r\n\r\n", 1)
                lines = header.split(b"\r\n")
                if lines[0] not in (b"HTTP/1.1 200 OK", b"HTTP/1.0 200 OK"):
                    raise EgressDenied("upstream status denied; redirects are never followed")
                lengths = []
                for line in lines[1:]:
                    name, sep, value = line.partition(b":")
                    if not sep or not name:
                        raise EgressDenied("malformed upstream header")
                    name = name.lower()
                    if name in (b"transfer-encoding", b"content-encoding"):
                        raise EgressDenied("encoded upstream body denied")
                    if name == b"content-length":
                        if not value.strip().isdigit():
                            raise EgressDenied("invalid upstream body length")
                        lengths.append(int(value.strip()))
                if len(lengths) != 1 or lengths[0] > limit or len(body) > lengths[0]:
                    raise EgressDenied("missing, conflicting or excessive upstream body length")
                while len(body) < lengths[0]:
                    chunk = recv_bound(min(4096, lengths[0] - len(body)))
                    if not chunk:
                        raise EgressDenied("truncated upstream response")
                    body += chunk
                return body
        except (OSError, ssl.SSLError) as exc:
            raise EgressDenied("TLS egress connection failed closed") from exc
        finally:
            if raw is not None:
                raw.close()
