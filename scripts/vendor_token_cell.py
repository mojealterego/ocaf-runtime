#!/usr/bin/env python3
"""Replace old paste macro inside pinned token-cell 2.1.1 with maintained pastey.

All source bytes are originally sourced through Cargo's checksum-validated
registry cache; a script patches exactly one normalized manifest section.
"""
from pathlib import Path
import argparse
import re
import shutil
import tomllib


def patch(repo: Path, cargo_home: Path) -> None:
    repo = repo.resolve()
    manifest = tomllib.loads((repo / "Cargo.toml").read_text())
    assert manifest["patch"]["crates-io"] == {
        "zenoh-transport": {"path": "vendor/zenoh-transport"}
    }, "unexpected pre-existing source patch"
    lock = tomllib.loads((repo / "Cargo.lock").read_text())["package"]
    candidates = [p for p in lock if p["name"] == "token-cell"]
    assert len(candidates) == 1 and candidates[0]["version"] == "2.1.1"
    assert candidates[0]["source"] == "registry+https://github.com/rust-lang/crates.io-index"
    assert re.fullmatch(r"[0-9a-f]{64}", candidates[0]["checksum"])
    sources = list((cargo_home / "registry/src").glob("*/token-cell-2.1.1"))
    assert len(sources) == 1, "expected exactly one checksum-verified token-cell source"
    target = repo / "vendor/token-cell"
    assert not target.exists(), "will not overwrite vendored token-cell"
    for file in sources[0].rglob("*"):
        assert not file.is_symlink(), "symlinks are forbidden in the vendored crate"
    shutil.copytree(sources[0], target)
    source_manifest = target / "Cargo.toml"
    source = source_manifest.read_text()
    section = re.search(
        r"(?ms)^\[dependencies\.paste\]\s*\n(.*?)(?=^\[|\Z)", source
    )
    assert section is not None, "normalized token-cell manifest lacks paste"
    original = section.group(1)
    assert "package =" not in original, "paste already aliases a different package"
    patch_section, changes = re.subn(
        r'(?m)^version = "(?:1|1\.0|1\.0\.15)"$',
        'version = "0.2.3"\npackage = "pastey"', original
    )
    assert changes == 1, "unexpected paste manifest version"
    patched = source[:section.start(1)] + patch_section + source[section.end(1):]
    source_manifest.write_text(patched)
    root = repo / "Cargo.toml"
    root.write_text(root.read_text().rstrip() +
        '\n# Compatibility backport: replace paste with maintained pastey (RUSTSEC-2024-0436).\n'
        'token-cell = { path = "vendor/token-cell" }\n')


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--source-home", type=Path, required=True)
    args = parser.parse_args()
    patch(args.repo, args.source_home)
