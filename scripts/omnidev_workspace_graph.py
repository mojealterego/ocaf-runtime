#!/usr/bin/env python3
"""Stage 0.56 OmniDev: extract the actual Cargo workspace dependency DAG.

Uses `cargo metadata --locked --offline --no-deps`, not regex-parsed Rust.
This is a package-level graph only: it does not provide symbol AST/CST or
prove dependency safety. The script executes no candidate patches or builds.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path
from typing import Any


class GraphError(ValueError):
    """Cargo graph cannot be established without ambiguity."""


def _inside(root: Path, path: Path) -> bool:
    return path == root or root in path.parents


def construct_graph(metadata: dict[str, Any], root: Path) -> dict[str, Any]:
    """Fail-closed workspace graph from a Cargo metadata v1 document."""
    root = root.resolve()
    if not isinstance(metadata, dict):
        raise GraphError("invalid cargo metadata")
    if Path(metadata.get("workspace_root", "")).resolve() != root:
        raise GraphError("workspace root mismatch")
    packages = metadata.get("packages")
    members = metadata.get("workspace_members")
    if not isinstance(packages, list) or not isinstance(members, list) or not members:
        raise GraphError("missing workspace graph")
    if len(members) != len(set(members)):
        raise GraphError("duplicate workspace member")
    all_pkgs = {p.get("id"): p for p in packages if isinstance(p, dict) and isinstance(p.get("id"), str)}
    if len(all_pkgs) != len(packages):
        raise GraphError("duplicate or malformed cargo package")
    if any(i not in all_pkgs for i in members):
        raise GraphError("missing workspace member package")

    by_name: dict[str, Path] = {}
    deps_by_name: dict[str, list[dict[str, Any]]] = {}
    for mid in members:
        p = all_pkgs[mid]
        name = p.get("name")
        manifest = p.get("manifest_path")
        dependencies = p.get("dependencies")
        if (not isinstance(name, str) or not name or not isinstance(manifest, str)
                or not isinstance(dependencies, list)):
            raise GraphError("invalid workspace member structure")
        if name in by_name:
            raise GraphError("duplicate package name")
        manifest_path = Path(manifest).resolve()
        if manifest_path.name != "Cargo.toml" or not _inside(root, manifest_path):
            raise GraphError("workspace manifest outside approved root")
        by_name[name] = manifest_path.parent
        deps_by_name[name] = dependencies

    path_to_name = {str(p): n for n, p in by_name.items()}
    if len(path_to_name) != len(by_name):
        raise GraphError("multiple workspace packages share one path")

    adjacency: dict[str, set[str]] = {n: set() for n in by_name}
    for name, dependencies in deps_by_name.items():
        for dep in dependencies:
            if not isinstance(dep, dict):
                raise GraphError("invalid cargo dependency")
            dep_path = dep.get("path")
            if dep_path is None:
                continue  # registry/git dependency: not an internal workspace edge
            if not isinstance(dep_path, str) or not dep_path:
                raise GraphError("invalid dependency path")
            target_path = Path(dep_path).resolve()
            if not _inside(root, target_path):
                raise GraphError("path dependency outside approved root")
            target = path_to_name.get(str(target_path))
            if target is None:
                raise GraphError("unresolved path dependency (workspace graph incomplete)")
            adjacency[name].add(target)

    order: list[str] = []
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(node: str) -> None:
        if node in visiting:
            raise GraphError("workspace dependency cycle")
        if node in visited:
            return
        visiting.add(node)
        for target in sorted(adjacency[node]):
            visit(target)
        visiting.remove(node)
        visited.add(node)
        order.append(node)

    for n in sorted(by_name):
        visit(n)

    graph = {
        "stage": "0.56",
        "schema_version": 1,
        "scope": "CARGO_WORKSPACE_PACKAGES_ONLY",
        "nodes": sorted(by_name),
        "edges": sorted(({"from": a, "to": b} for a in adjacency for b in adjacency[a]),
                        key=lambda e: (e["from"], e["to"])),
        "topological_order": order,
    }
    canonical = json.dumps(graph, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    graph["graph_sha256"] = hashlib.sha256(canonical).hexdigest()
    return graph


def build_graph(repo: Path) -> dict[str, Any]:
    repo = repo.resolve(strict=True)
    if not (repo / "Cargo.toml").is_file() or not (repo / "Cargo.lock").is_file():
        raise GraphError("Cargo.toml or Cargo.lock missing")
    try:
        result = subprocess.run(
            ["cargo", "metadata", "--locked", "--offline", "--no-deps", "--format-version", "1"],
            cwd=repo, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            timeout=30, check=False, env={**os.environ, "CARGO_NET_OFFLINE": "true"},
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise GraphError("cargo metadata unavailable or timed out") from exc
    if result.returncode != 0:
        raise GraphError("cargo metadata failed: " + result.stderr.decode("utf-8", "replace")[:300])
    if len(result.stdout) > 8 * 1024 * 1024:
        raise GraphError("cargo metadata exceeds result limit")
    try:
        metadata = json.loads(result.stdout)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise GraphError("cargo metadata is not valid JSON") from exc
    return construct_graph(metadata, repo)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True, type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(build_graph(args.repo), sort_keys=True, ensure_ascii=False))
        return 0
    except (GraphError, OSError) as exc:
        print(json.dumps({"status": "DENIED", "reason": str(exc)}, sort_keys=True))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
