#!/usr/bin/env python3
"""Reject missing, empty or wrong-format native build artifacts in CI."""
import argparse
from pathlib import Path
import struct
import sys
import zipfile

MACHO_MAGICS = {
    bytes.fromhex("feedface"), bytes.fromhex("cefaedfe"),
    bytes.fromhex("feedfacf"), bytes.fromhex("cffaedfe"),
    bytes.fromhex("cafebabe"), bytes.fromhex("bebafeca"),
}


def verify(kind: str, path: Path) -> None:
    if not path.is_file() or path.stat().st_size < 1024:
        raise ValueError(f"Missing or implausibly small {kind} artifact: {path}")
    with path.open("rb") as file:
        header = file.read(4096)
        if kind == "windows":
            if header[:2] != b"MZ":
                raise ValueError("Windows executable lacks MZ signature")
            pe_offset = struct.unpack_from("<I", header, 0x3C)[0]
            file.seek(pe_offset)
            if file.read(4) != bytes.fromhex("50450000"):
                raise ValueError("Windows executable lacks PE signature")
        elif kind == "linux":
            if header[:4] != bytes.fromhex("7f454c46"):
                raise ValueError("Linux executable lacks ELF signature")
            if header[4] not in (1, 2) or header[5] not in (1, 2):
                raise ValueError("Linux executable has invalid ELF class or byte order")
            if header[16:18] == bytes.fromhex("0000"):
                raise ValueError("Linux ELF has no object type")
        elif kind in {"macos", "ios"}:
            if header[:4] not in MACHO_MAGICS:
                raise ValueError(f"{kind} executable lacks Mach-O signature")
        elif kind == "android":
            if not zipfile.is_zipfile(path):
                raise ValueError("APK is not a ZIP archive")
            with zipfile.ZipFile(path) as apk:
                if "AndroidManifest.xml" not in apk.namelist():
                    raise ValueError("APK lacks AndroidManifest.xml")
                dex_files = [entry for entry in apk.infolist() if entry.filename.endswith(".dex") and not entry.is_dir()]
                if not dex_files:
                    raise ValueError("APK lacks DEX bytecode")
                for entry in dex_files:
                    if entry.file_size < 112:
                        raise ValueError(f"APK contains truncated DEX bytecode: {entry.filename}")
                    with apk.open(entry) as dex:
                        magic = dex.read(8)
                    if not (magic[:4] == b"dex\n" and magic[4:7] in {b"035", b"037", b"038", b"039", b"040", b"041"} and magic[7:] == b"\x00"):
                        raise ValueError(f"APK contains invalid DEX header: {entry.filename}")
        else:
            raise ValueError(f"Unsupported artifact kind: {kind}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("kind", choices=["windows", "macos", "android", "ios", "linux"])
    parser.add_argument("path", type=Path)
    args = parser.parse_args()
    try:
        verify(args.kind, args.path)
    except (OSError, ValueError, struct.error, zipfile.BadZipFile) as exc:
        print(f"Artifact verification FAILED: {exc}", file=sys.stderr)
        sys.exit(1)
    print(f"Artifact verification OK: {args.kind} {args.path}")
