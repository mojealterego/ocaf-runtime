#!/usr/bin/env python3
"""Linux/Xvfb Tauri window smoke: require an actual mapped window and PNG evidence."""
import argparse
from pathlib import Path
import subprocess
import struct
import zlib
import time



def _valid_png(data: bytes) -> bool:
    """Validate bounded PNG chunk structure, CRCs and a terminal IEND."""
    if len(data) < 1024 or not data.startswith(bytes.fromhex("89504e470d0a1a0a")):
        return False
    pos = 8
    seen_ihdr = seen_idat = seen_iend = after_idat = seen_plte = False
    while pos < len(data):
        if len(data) - pos < 12:
            return False
        length = struct.unpack_from(">I", data, pos)[0]
        pos += 4
        kind = data[pos:pos + 4]
        pos += 4
        if length > len(data) - pos - 4:
            return False
        payload = data[pos:pos + length]
        pos += length
        crc = struct.unpack_from(">I", data, pos)[0]
        pos += 4
        if zlib.crc32(kind + payload) & 0xffffffff != crc:
            return False
        if not seen_ihdr:
            if kind != b"IHDR" or length != 13:
                return False
            width, height, depth, color, compression, filter_method, interlace = struct.unpack(">IIBBBBB", payload)
            if (width < 320 or height < 240 or depth not in (1, 2, 4, 8, 16)
                    or color not in (0, 2, 3, 4, 6) or compression != 0
                    or filter_method != 0 or interlace not in (0, 1)):
                return False
            if (color in (2, 4, 6) and depth not in (8, 16)) or (color == 3 and depth == 16):
                return False
            seen_ihdr = True
        elif kind == b"IHDR":
            return False
        elif kind == b"PLTE":
            if seen_plte or seen_idat or length == 0 or length > 768 or length % 3:
                return False
            seen_plte = True
        elif kind == b"IDAT":
            if after_idat or length == 0 or (color == 3 and not seen_plte):
                return False
            seen_idat = True
        elif kind == b"IEND":
            if length != 0 or not seen_idat or pos != len(data):
                return False
            seen_iend = True
            break
        else:
            if seen_idat:
                after_idat = True
            # Unknown critical chunks are not safe to accept.
            if not kind or kind[0] & 32 == 0:
                return False
    return seen_ihdr and seen_idat and seen_iend


def run(binary: Path, screenshot: Path, seconds: float = 15.0) -> None:
    if not binary.is_file():
        raise FileNotFoundError(binary)
    process = subprocess.Popen([str(binary)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        deadline = time.monotonic() + seconds
        window = None
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError("Linux desktop process exited before mapping its window")
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                break
            try:
                result = subprocess.run(
                    ["xdotool", "search", "--onlyvisible", "--name", "OCAF Agents Builder"],
                    capture_output=True, text=True, timeout=min(5.0, remaining), check=False,
                )
            except subprocess.TimeoutExpired:
                # Window discovery must not outlive the shared GUI smoke deadline.
                continue
            if result.returncode == 0 and result.stdout.strip():
                for candidate in result.stdout.splitlines():
                    if process.poll() is not None:
                        raise RuntimeError("Linux desktop process exited before mapping its window")
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        break
                    try:
                        owner = subprocess.run(
                            ["xdotool", "getwindowpid", candidate.strip()],
                            capture_output=True, text=True, timeout=min(5.0, remaining), check=False,
                        )
                    except subprocess.TimeoutExpired:
                        continue
                    if owner.returncode == 0 and owner.stdout.strip() == str(process.pid):
                        window = candidate.strip()
                        break
                if window is not None:
                    break
            time.sleep(min(0.5, max(0.0, deadline - time.monotonic())))
        if window is None:
            raise RuntimeError("No visible OCAF Agents Builder window detected")
        screenshot.parent.mkdir(parents=True, exist_ok=True)
        # A mapped X11 window may not have completed its first paint.
        # Keep all PNG checks strict; retry only within the original deadline.
        while True:
            if process.poll() is not None:
                raise RuntimeError("Linux desktop process exited before screenshot validation")
            screenshot.unlink(missing_ok=True)
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise RuntimeError("Missing or invalid Linux window screenshot")
            try:
                subprocess.run(
                    ["import", "-window", window, str(screenshot)],
                    check=True,
                    timeout=min(5.0, remaining),
                )
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
                # A transient X11 capture failure is not evidence of a valid image.
                # Retry only while the original smoke deadline and process permit.
                pass
            data = screenshot.read_bytes() if screenshot.is_file() else b""
            if _valid_png(data):
                break
            if time.monotonic() >= deadline:
                raise RuntimeError("Missing or invalid Linux window screenshot")
            time.sleep(min(0.5, max(0.0, deadline - time.monotonic())))
    finally:
        if process.poll() is None:
            process.terminate()
        try:
            process.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.communicate(timeout=5)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("binary", type=Path)
    parser.add_argument("screenshot", type=Path)
    args = parser.parse_args()
    run(args.binary, args.screenshot)
    print("Linux GUI window and PNG smoke PASSED")
