#!/usr/bin/env python3
"""Build a tenant and run specific WASIp1 fd_write module as WAT source.

The nonce is public challenge data. Its integrity comes from the module digest
and the guest rootfs digest in the signed admission manifest. The guest PID1
must independently compare the exact output and emit its own serial receipt.
"""
from __future__ import annotations

import argparse
from pathlib import Path
import re

NONCE = re.compile(r"[0-9a-f]{32}\Z")
TENANTS = frozenset({"tenant-a", "tenant-b"})
MARKER = "OCAF039_WASI_INSIDE_FC_VM_IMPORT_FD_WRITE"


def expected_output(tenant: str, nonce: str) -> str:
    if tenant not in TENANTS or NONCE.fullmatch(nonce) is None:
        raise ValueError("invalid WASI tenant or challenge nonce")
    return f"{MARKER}:{tenant}:{nonce}\n"


def render_wat(tenant: str, nonce: str) -> str:
    output = expected_output(tenant, nonce)
    # These validated values contain only printable ASCII and LF. WAT \0a is
    # the only escape; no user supplied text can alter the module structure.
    wat_data = output[:-1] + r"\0a"
    return f'''(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 64) "{wat_data}")
  (func (export "_start")
    (i32.store (i32.const 0) (i32.const 64))
    (i32.store (i32.const 4) (i32.const {len(output.encode("ascii"))}))
    (if (i32.ne
          (call $fd_write (i32.const 1) (i32.const 0)
                         (i32.const 1) (i32.const 16))
          (i32.const 0))
      (then unreachable))
  )
)
'''


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tenant", choices=sorted(TENANTS))
    parser.add_argument("nonce")
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    try:
        wat = render_wat(args.tenant, args.nonce)
    except ValueError as exc:
        parser.error(str(exc))
    args.output.write_text(wat, encoding="ascii")


if __name__ == "__main__":
    main()
