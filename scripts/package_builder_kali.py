#!/usr/bin/env python3
"""Package a verified Linux x86-64 OCAF Agents Builder ELF for Kali Linux.

This builds an unsigned Debian-format package from an actual native executable.
It does not build Rust, sign the package, or attest a running Kali device.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import struct
import subprocess
import tempfile
import zlib

PACKAGE = "ocaf-agents-builder"
BINARY = "ocaf-agents-builder-desktop"
ID = "org.ocaf.agentsbuilder"
EPOCH = 315532800
MAX_BINARY = 512 * 1024 * 1024
MAX_ICON = 1024 * 1024


def regular_file(path: Path, *, limit: int, minimum: int = 1024) -> bytes:
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_size < minimum or info.st_size > limit:
        raise ValueError(f"invalid regular input file: {path}")
    fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        fresh = os.fstat(fd)
        if not stat.S_ISREG(fresh.st_mode) or fresh.st_size != info.st_size or fresh.st_ino != info.st_ino:
            raise ValueError("input file changed during validation")
        with os.fdopen(os.dup(fd), "rb") as file:
            data = file.read(limit + 1)
        if len(data) != fresh.st_size:
            raise ValueError("input file changed during read")
    finally:
        os.close(fd)
    return data


def validate_linux_amd64(binary: bytes) -> None:
    if len(binary) < 64 or binary[:4] != b"\x7fELF":
        raise ValueError("expected a Linux ELF executable")
    if binary[4:6] != b"\x02\x01":
        raise ValueError("expected little-endian ELF64")
    e_type, e_machine = struct.unpack_from("<HH", binary, 16)
    if e_type not in (2, 3) or e_machine != 62:
        raise ValueError("expected x86-64 executable or PIE ELF")
    if struct.unpack_from("<I", binary, 20)[0] != 1:
        raise ValueError("invalid ELF version")


def validate_icon(png: bytes) -> None:
    if not png.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError("icon is not PNG")
    offset = 8
    seen_ihdr = seen_idat = seen_iend = False
    while offset < len(png):
        if len(png) - offset < 12:
            raise ValueError("truncated PNG chunk")
        size = struct.unpack_from(">I", png, offset)[0]
        end = offset + 12 + size
        if size > MAX_ICON or end > len(png):
            raise ValueError("truncated PNG chunk")
        kind = png[offset + 4:offset + 8]
        payload = png[offset + 8:offset + 8 + size]
        checksum = struct.unpack_from(">I", png, offset + 8 + size)[0]
        if zlib.crc32(kind + payload) & 0xffffffff != checksum:
            raise ValueError("PNG CRC mismatch")
        if kind == b"IHDR":
            if seen_ihdr or offset != 8 or size != 13:
                raise ValueError("invalid PNG IHDR")
            width, height = struct.unpack_from(">II", payload)
            if not (1 <= width <= 4096 and 1 <= height <= 4096):
                raise ValueError("invalid PNG dimensions")
            seen_ihdr = True
        elif kind == b"IDAT":
            if not seen_ihdr:
                raise ValueError("PNG IDAT before IHDR")
            seen_idat = True
        elif kind == b"IEND":
            if not seen_idat or size or end != len(png):
                raise ValueError("invalid PNG IEND or trailing data")
            seen_iend = True
        elif not seen_ihdr or seen_iend:
            raise ValueError("invalid PNG chunk ordering")
        offset = end
    if not (seen_ihdr and seen_idat and seen_iend):
        raise ValueError("incomplete PNG")



def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def package_kali(binary_path: Path, icon_path: Path, output: Path,
                 *, version: str, commit: str, timestamp: int = EPOCH) -> dict[str, str]:
    if not re.fullmatch(r"[0-9]+(?:\.[0-9]+){1,3}", version):
        raise ValueError("invalid Debian package version")
    if not re.fullmatch(r"[a-f0-9]{40}", commit):
        raise ValueError("invalid source commit SHA")
    if timestamp < EPOCH or timestamp > 4102444800:
        raise ValueError("invalid reproducible build timestamp")
    binary = regular_file(binary_path, limit=MAX_BINARY)
    validate_linux_amd64(binary)
    icon = regular_file(icon_path, limit=MAX_ICON, minimum=67)
    validate_icon(icon)
    if not shutil.which("dpkg-deb"):
        raise RuntimeError("dpkg-deb is required")
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.is_symlink() or output.exists():
        raise FileExistsError(f"refusing to overwrite package: {output}")

    with tempfile.TemporaryDirectory(prefix=".ocaf-kali-", dir=output.parent) as tmp:
        staging = Path(tmp)
        root = staging / "root"
        control = root / "DEBIAN"
        bin_dst = root / "usr/bin" / PACKAGE
        icon_dst = root / "usr/share/icons/hicolor/256x256/apps" / (ID + ".png")
        desktop_dst = root / "usr/share/applications" / (ID + ".desktop")
        for directory in (control, bin_dst.parent, icon_dst.parent, desktop_dst.parent):
            directory.mkdir(parents=True, exist_ok=True)
        for directory in sorted(root.rglob("*")):
            if directory.is_dir():
                directory.chmod(0o755)
        (control / "control").write_text(
            f"Package: {PACKAGE}\nVersion: {version}\nSection: utils\n"
            "Priority: optional\nArchitecture: amd64\n"
            "Maintainer: OCAF Project <noreply@ocaf.invalid>\n"
            "Depends: libc6 (>= 2.39), libwebkit2gtk-4.1-0, libgtk-3-0t64 | libgtk-3-0, libglib2.0-0t64 | libglib2.0-0, libssl3t64 | libssl3\n"
            "Description: OCAF Agents Builder Code/NoCode with Local/Cloud switching\n"
            " Unsigned native application package for Kali Linux.\n",
            encoding="utf-8",
        )
        bin_dst.write_bytes(binary)
        bin_dst.chmod(0o755)
        icon_dst.write_bytes(icon)
        icon_dst.chmod(0o644)
        desktop_dst.write_text(
            "[Desktop Entry]\nType=Application\nName=OCAF Agents Builder\n"
            f"Exec=/usr/bin/{PACKAGE}\nIcon={ID}\n"
            "Categories=Development;\nTerminal=false\nStartupNotify=true\n",
            encoding="utf-8",
        )
        (control / "control").chmod(0o644)
        desktop_dst.chmod(0o644)
        for path in sorted(root.rglob("*"), reverse=True):
            os.utime(path, (timestamp, timestamp), follow_symlinks=False)
        os.utime(root, (timestamp, timestamp))
        env = {**os.environ, "SOURCE_DATE_EPOCH": str(timestamp)}
        temporary_deb = staging / "package.deb"
        subprocess.run(
            ["dpkg-deb", "--build", "--root-owner-group",
             "--compression=gzip", "--compression-level=9", str(root), str(temporary_deb)],
            env=env, check=True, capture_output=True, timeout=120,
        )
        os.link(temporary_deb, output)
    return {"source_commit": commit, "package": str(output),
            "package_sha256": sha256(output.read_bytes()),
            "binary_sha256": sha256(binary), "icon_sha256": sha256(icon),
            "platform": "kali-linux-amd64", "signing": "UNSIGNED",
            "device_e2e": "UNVERIFIED"}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--icon", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--timestamp", type=int, default=EPOCH)
    args = parser.parse_args()
    evidence = package_kali(args.binary, args.icon, args.out, version=args.version,
                            commit=args.commit, timestamp=args.timestamp)
    print(json.dumps(evidence, sort_keys=True))


if __name__ == "__main__":
    main()
