#!/usr/bin/env python3
"""Deterministically create minimal native icon resources without external dependencies."""
from pathlib import Path
import struct
import zlib

SIZE = 64
BG = (7, 8, 10, 255)
GOLD = (212, 175, 55, 255)


def chunk(tag: bytes, payload: bytes) -> bytes:
    return struct.pack(">I", len(payload)) + tag + payload + struct.pack(">I", zlib.crc32(tag + payload) & 0xffffffff)


def png() -> bytes:
    rows = bytearray()
    for y in range(SIZE):
        rows.append(0)
        for x in range(SIZE):
            border = 9 <= x < SIZE - 9 and 9 <= y < SIZE - 9
            stroke = border and (x in (9, SIZE - 10) or y in (9, SIZE - 10))
            rows.extend(GOLD if stroke else BG)
    ihdr = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)
    return bytes.fromhex("89504e470d0a1a0a") + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(bytes(rows), 9)) + chunk(b"IEND", b"")


def main() -> None:
    directory = Path(__file__).resolve().parents[1] / "apps/agents-builder-desktop/src-tauri/icons"
    directory.mkdir(parents=True, exist_ok=True)
    picture = png()
    (directory / "icon.png").write_bytes(picture)
    # ICO directory: one 64x64 PNG image, 32-bit RGBA.
    header = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack("<BBBBHHII", SIZE, SIZE, 0, 0, 1, 32, len(picture), 22)
    (directory / "icon.ico").write_bytes(header + entry + picture)


if __name__ == "__main__":
    main()
