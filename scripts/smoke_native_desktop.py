#!/usr/bin/env python3
"""Start a native desktop executable and verify it remains alive briefly.

This is a process smoke test, not a GUI interaction test.
"""
import argparse
from pathlib import Path
import subprocess
import sys
import time


def smoke(path: Path, seconds: float = 5.0) -> None:
    if not path.is_file():
        raise FileNotFoundError(path)
    process = subprocess.Popen([str(path)], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    try:
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            status = process.poll()
            if status is not None:
                output = process.stderr.read().decode("utf-8", errors="replace")[-4000:]
                raise RuntimeError(f"Desktop application exited prematurely: {status}; {output}")
            time.sleep(0.25)
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
    parser.add_argument("executable", type=Path)
    parser.add_argument("--seconds", type=float, default=5)
    args = parser.parse_args()
    if not 0.5 <= args.seconds <= 60:
        parser.error("--seconds must be between 0.5 and 60")
    try:
        smoke(args.executable, args.seconds)
    except (OSError, RuntimeError) as error:
        print(f"Desktop launch smoke FAILED: {error}", file=sys.stderr)
        sys.exit(1)
    print("Desktop launch smoke PASSED")
