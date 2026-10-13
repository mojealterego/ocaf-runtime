#!/usr/bin/env python3
"""Reproducible vendor-only security backport from verified Cargo registry source.

Only modifies zenoh-transport's normalized Cargo.toml dependency on lz4_flex.
Source archive is initially verified by Cargo against the existing lockfile.
"""
import argparse
from pathlib import Path
import re
import shutil
import stat
import tomllib


def vendor(repo: Path, home: Path) -> None:
    repo = repo.resolve()
    manifest = tomllib.loads((repo / "Cargo.toml").read_text())
    assert "patch" not in manifest, "refusing to overwrite an existing source patch"
    packages = tomllib.loads((repo / "Cargo.lock").read_text())["package"]
    current = [p for p in packages if p["name"] == "zenoh-transport"]
    assert len(current) == 1 and current[0]["version"] == "1.10.1"
    assert current[0]["source"] == "registry+https://github.com/rust-lang/crates.io-index"
    assert re.fullmatch(r"[0-9a-f]{64}", current[0]["checksum"])
    source_paths = list((home / "registry/src").glob("*/zenoh-transport-1.10.1"))
    assert len(source_paths) == 1, "a unique Cargo-verifed source directory is required"
    target = repo / "vendor/zenoh-transport"
    assert not target.exists(), "will not overwrite existing vendored source"
    for path in source_paths[0].rglob("*"):
        assert not path.is_symlink(), "registry source must not contain symlinks"
        assert path.is_dir() or path.is_file(), "unexpected registry source type"
    shutil.copytree(source_paths[0], target)
    for path in target.rglob("*"):
        if path.is_file():
            path.chmod(stat.S_IMODE(path.stat().st_mode) & ~0o222 | 0o644)
    cargo = target / "Cargo.toml"
    old = cargo.read_text()
    section = re.search(
        r"(?ms)^\[dependencies\.lz4_flex\]\s*\n(.*?)(?=^\[|\Z)", old
    )
    assert section is not None, "normalized registry manifest missing lz4_flex"
    original = section.group(1)
    updated = re.sub(r'(?m)^version = "0\.10\.0"$',
                     'version = "0.11.6"', original)
    assert updated != original and updated.count('version = "0.11.6"') == 1
    patched = old[:section.start(1)] + updated + old[section.end(1):]
    cargo.write_text(patched)
    # Keep upstream package content, original metadata and license files intact.
    root = repo / "Cargo.toml"
    root.write_text(root.read_text().rstrip() +
        '\n\n# Reviewed single-crate security backport (RUSTSEC-2026-0041).\n'
        '[patch.crates-io]\n'
        'zenoh-transport = { path = "vendor/zenoh-transport" }\n')


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--source-home", type=Path, required=True)
    args = parser.parse_args()
    vendor(args.repo, args.source_home)
