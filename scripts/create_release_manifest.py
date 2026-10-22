#!/usr/bin/env python3
"""Create a deterministic source binding for an OCAF release bundle.

The manifest is evidence metadata, not a signature.  GitHub attestation (or an
equivalent trusted signer) must authenticate the archive and SBOM separately.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import sys
from pathlib import Path


SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_SHA = re.compile(r"[0-9a-f]{40}\Z")
REPOSITORY = re.compile(r"[A-Za-z0-9_.-]{1,100}/[A-Za-z0-9_.-]{1,100}\Z")
SUBJECTS = ("ocaf_runtime-linux-x86_64.tar.gz", "ocaf_runtime.cdx.json")
MAX_SUBJECT_BYTES = 256 * 1024 * 1024


class ManifestError(ValueError):
    """The requested release source binding is unsafe or incomplete."""


def _digest(path: Path) -> str:
    try:
        metadata = path.lstat()
    except OSError as error:
        raise ManifestError(f"release subject missing: {path.name}") from error
    if not stat.S_ISREG(metadata.st_mode) or not 0 < metadata.st_size <= MAX_SUBJECT_BYTES:
        raise ManifestError(f"release subject is not a bounded regular file: {path.name}")
    result = hashlib.sha256()
    try:
        with path.open("rb") as content:
            for block in iter(lambda: content.read(1024 * 1024), b""):
                result.update(block)
    except OSError as error:
        raise ManifestError(f"cannot hash release subject: {path.name}") from error
    digest = result.hexdigest()
    if SHA256.fullmatch(digest) is None:  # defensive invariant
        raise ManifestError("invalid SHA-256 result")
    return digest


def build_manifest(directory: Path, git_sha: str, repository: str) -> dict[str, object]:
    if GIT_SHA.fullmatch(git_sha) is None:
        raise ManifestError("git SHA must be exactly 40 lowercase hexadecimal characters")
    if REPOSITORY.fullmatch(repository) is None:
        raise ManifestError("repository must be an owner/name GitHub repository")
    root = directory.absolute()
    if not root.is_dir():
        raise ManifestError("release directory does not exist")
    subjects = {name: {"sha256": _digest(root / name)} for name in SUBJECTS}
    return {
        "schema_version": 1,
        "source": {
            "git_sha": git_sha,
            "ref": "refs/heads/main",
            "repository": repository,
            "workflow": ".github/workflows/supply-chain.yml",
        },
        "subjects": subjects,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dir", type=Path, required=True)
    parser.add_argument("--git-sha", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--output", default="RELEASE-MANIFEST.json")
    args = parser.parse_args(argv)
    if args.output != "RELEASE-MANIFEST.json":
        parser.error("release manifest filename is fixed")
    try:
        manifest = build_manifest(args.dir, args.git_sha, args.repository)
        output = args.dir.absolute() / args.output
        payload = (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode("utf-8")
        flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
        flags |= getattr(os, "O_NOFOLLOW", 0)
        try:
            descriptor = os.open(output, flags, 0o600)
        except OSError as error:
            raise ManifestError("release manifest output already exists or is unsafe") from error
        try:
            with os.fdopen(descriptor, "wb") as stream:
                stream.write(payload)
        except OSError as error:
            try:
                output.unlink(missing_ok=True)
            except OSError:
                pass
            raise ManifestError("cannot write release manifest") from error
    except (ManifestError, OSError) as error:
        print(f"release manifest creation failed: {error}", file=sys.stderr)
        return 2
    print(json.dumps({"status": "CREATED", "path": str(output)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
