#!/usr/bin/env python3
"""OmniDev 0.56: real Rust Tree-sitter CST indexing; conservative lexical impact.

CST parsing is not a proof of type resolution or compilation. No candidate code
is executed. Parsing fails closed on invalid syntax, oversize files and symlinks.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath

MAX_FILE_BYTES = 1024 * 1024
MAX_NODES = 100_000
MAX_FILES = 512


class SyntaxIndexError(ValueError):
    """Input must not be used for automated patching."""


def _parser():
    try:
        import tree_sitter_rust
        from tree_sitter import Language, Parser
        return Parser(Language(tree_sitter_rust.language()))
    except (ImportError, TypeError, ValueError) as exc:
        raise SyntaxIndexError("pinned Tree-sitter Rust grammar not available") from exc


def _normal_path(path: str) -> str:
    pp = PurePosixPath(path)
    if (not isinstance(path, str) or not path.endswith(".rs")
            or pp.is_absolute() or len(pp.parts) < 2
            or any(p in ("", ".", "..") or p.startswith(".") for p in pp.parts)
            or not path.startswith(("src/", "tests/", "crates/", "swarms/"))):
        raise SyntaxIndexError("Rust source outside allowed roots")
    return path


def _walk(root):
    pending = [root]
    count = 0
    while pending:
        node = pending.pop()
        count += 1
        if count > MAX_NODES:
            raise SyntaxIndexError("CST exceeds node limit")
        yield node
        pending.extend(reversed(node.children))


def parse_rust(data: bytes) -> dict:
    if not isinstance(data, bytes) or len(data) > MAX_FILE_BYTES:
        raise SyntaxIndexError("Rust source exceeds size limit")
    if b"\x00" in data:
        raise SyntaxIndexError("binary Rust source")
    try:
        data.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise SyntaxIndexError("non UTF-8 Rust source") from exc
    tree = _parser().parse(data)
    if tree.root_node.has_error:
        raise SyntaxIndexError("Rust syntax contains ERROR or MISSING")
    symbols = []
    references = set()
    kinds = {"function_item", "struct_item", "enum_item",
             "trait_item", "mod_item", "type_item", "const_item", "static_item"}
    for node in _walk(tree.root_node):
        if node.type in kinds:
            name = node.child_by_field_name("name")
            if name is not None:
                symbols.append({"name": data[name.start_byte:name.end_byte].decode("utf-8"),
                                "kind": node.type, "line": node.start_point.row + 1})
        elif node.type == "call_expression":
            fn = node.child_by_field_name("function")
            if fn is not None:
                raw = data[fn.start_byte:fn.end_byte].decode("utf-8")
                references.add(raw.split("::")[-1])
        elif node.type == "use_declaration":
            raw = data[node.start_byte:node.end_byte].decode("utf-8")
            references.update(token for token in
                              __import__("re").findall(r"[A-Za-z_][A-Za-z_0-9]*", raw)
                              if token != "use")
    return {"symbols": sorted(symbols, key=lambda s: (s["line"], s["name"])),
            "references": sorted(references),
            "sha256": hashlib.sha256(data).hexdigest()}


class RustSymbolIndex:
    """Only changed files are reparsed; removed files are evicted."""

    def __init__(self, root: Path):
        self.root = root.resolve(strict=True)
        self.files: dict[str, dict] = {}

    def update(self, paths: list[str]) -> dict[str, list[str]]:
        if len(paths) > MAX_FILES or len(paths) != len(set(paths)):
            raise SyntaxIndexError("invalid incremental batch")
        changed, removed = [], []
        # Parse into a shadow map; a rejected batch must not partially publish.
        replacement = dict(self.files)
        for name in paths:
            _normal_path(name)
            target = self.root / name
            if any(p.is_symlink() for p in (target, *list(target.parents)[:-1])
                   if p == self.root or self.root in p.parents):
                raise SyntaxIndexError("source path traverses a symlink")
            if not target.exists():
                if name in replacement:
                    replacement.pop(name)
                    removed.append(name)
                continue
            if not target.is_file():
                raise SyntaxIndexError("not a Rust source file")
            try:
                with target.open("rb") as stream:
                    data = stream.read(MAX_FILE_BYTES + 1)
            except OSError as exc:
                raise SyntaxIndexError("Rust source cannot be read") from exc
            digest = hashlib.sha256(data).hexdigest()
            if replacement.get(name, {}).get("sha256") == digest:
                continue
            replacement[name] = parse_rust(data)
            changed.append(name)
        self.files = replacement
        return {"changed": sorted(changed), "removed": sorted(removed)}

    def impacted(self, changed: list[str]) -> dict[str, list[str]]:
        if not changed or any(name not in self.files for name in changed):
            raise SyntaxIndexError("unknown changed Rust source")
        names = {s["name"] for p in changed for s in self.files[p]["symbols"]}
        dependents = sorted(p for p, record in self.files.items()
                            if p not in changed and names.intersection(record["references"]))
        return {"changed": sorted(changed), "potential_dependents": dependents,
                "scope": "CONSERVATIVE_SYMBOL_NAME_MATCH_NOT_TYPE_RESOLUTION"}

    def fingerprint(self) -> str:
        canonical = json.dumps(self.files, sort_keys=True, separators=(",", ":")).encode()
        return hashlib.sha256(canonical).hexdigest()
