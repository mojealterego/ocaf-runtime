#!/usr/bin/env python3
"""Validate KaiOS 2.5 package contents and security boundaries."""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile

EXPECTED = {"manifest.webapp", "index.html", "app.js", "style.css", "canonical-graph.js", "canonical-ui.js"}
MAX_ENTRY_BYTES = 64 * 1024


def verify(path: Path) -> dict:
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        if len(names) != len(EXPECTED) or set(names) != EXPECTED:
            raise ValueError("Unexpected, missing or duplicate ZIP members")
        for info in archive.infolist():
            if info.is_dir() or info.file_size > MAX_ENTRY_BYTES:
                raise ValueError("Unexpected directory or oversized member")
        if archive.testzip() is not None:
            raise ValueError("Corrupted ZIP member")
        manifest = json.loads(archive.read("manifest.webapp"))
        if manifest.get("launch_path") != "/index.html" or manifest.get("type") != "web":
            raise ValueError("Unsupported KaiOS launch configuration")
        if manifest.get("permissions") != {}:
            raise ValueError("Unexpected privileged KaiOS permissions")
        if (b'<script src="app.js"></script>' not in archive.read("index.html") or
                b'<script src="canonical-graph.js"></script>' not in archive.read("index.html")):
            raise ValueError("App entrypoint not referenced")
        if b'<script src="canonical-ui.js"></script>' not in archive.read("index.html"):
            raise ValueError("Canonical UI module missing from entrypoint")
        return {name: hashlib.sha256(archive.read(name)).hexdigest() for name in sorted(EXPECTED)}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("archive", type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(args.archive), indent=2))


if __name__ == "__main__":
    main()
