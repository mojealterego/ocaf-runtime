#!/usr/bin/env python3
"""Linux/Xvfb Tauri window smoke: require an actual mapped window and PNG evidence."""
import argparse
from pathlib import Path
import subprocess
import time


def run(binary: Path, screenshot: Path, seconds: float = 15.0) -> None:
    if not binary.is_file():
        raise FileNotFoundError(binary)
    process = subprocess.Popen([str(binary)], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    try:
        deadline = time.monotonic() + seconds
        window = None
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError("Linux desktop process exited before mapping its window")
            result = subprocess.run(
                ["xdotool", "search", "--onlyvisible", "--name", "OCAF Agents Builder"],
                capture_output=True, text=True, timeout=5, check=False,
            )
            if result.returncode == 0 and result.stdout.strip():
                window = result.stdout.splitlines()[0].strip()
                break
            time.sleep(0.5)
        if window is None:
            raise RuntimeError("No visible OCAF Agents Builder window detected")
        screenshot.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["import", "-window", window, str(screenshot)], check=True, timeout=15)
        if screenshot.stat().st_size < 1024 or screenshot.read_bytes()[:8] != bytes.fromhex("89504e470d0a1a0a"):
            raise RuntimeError("Missing or invalid Linux window screenshot")
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
