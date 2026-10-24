"""Derive a bounded W3C traceparent from an authenticated stage 0.39 lease."""
from __future__ import annotations

import hashlib
import re

TRACEPARENT = re.compile(r"00-[0-9a-f]{32}-[0-9a-f]{16}-01\Z")
REQUEST_ID = re.compile(r"[0-9a-f]{32}\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
TENANT = re.compile(r"tenant-[ab]\Z")


def is_valid(value: object) -> bool:
    return (isinstance(value,str) and TRACEPARENT.fullmatch(value) is not None
            and int(value[3:35],16)!=0 and int(value[36:52],16)!=0)


def derive(request_id: str, manifest_sha256: str, tenant_id: str) -> str:
    """Bind trace and span identity to one kernel-authenticated lease."""
    if (not isinstance(request_id, str) or REQUEST_ID.fullmatch(request_id) is None
        or not isinstance(manifest_sha256, str) or SHA256.fullmatch(manifest_sha256) is None
        or not isinstance(tenant_id, str) or TENANT.fullmatch(tenant_id) is None):
        raise ValueError("invalid broker trace inputs")
    seed = (b"ocaf039-broker-trace-v1\0" + request_id.encode() + b"\0" +
            manifest_sha256.encode() + b"\0" + tenant_id.encode())
    trace_id = hashlib.sha256(seed + b"\0trace").hexdigest()[:32]
    span_id = hashlib.sha256(seed + b"\0span").hexdigest()[:16]
    traceparent = f"00-{trace_id}-{span_id}-01"
    if not is_valid(traceparent):
        raise ValueError("invalid derived traceparent")
    return traceparent
