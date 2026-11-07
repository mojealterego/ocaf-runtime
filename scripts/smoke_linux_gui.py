#!/usr/bin/env python3
"""Linux/Xvfb Tauri window smoke: require an actual mapped window and PNG evidence."""
import argparse
from pathlib import Path
import subprocess
import struct
import time


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
            valid_header = (
                len(data) >= 1024
                and data[:8] == bytes.fromhex("89504e470d0a1a0a")
                and data[12:16] == b"IHDR"
                and struct.unpack(">I", data[8:12])[0] == 13
            )
            if valid_header:
                width, height = struct.unpack(">II", data[16:24])
                if width >= 320 and height >= 240:
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
