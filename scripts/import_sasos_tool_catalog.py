#!/usr/bin/env python3
"""Validate the supplied Rust OMAt registry and export inert catalog metadata."""

import argparse
import json
import re
from pathlib import Path


STRING = r'"(?:\\.|[^"\\])*"'
IMPL = re.compile(
    r"impl\s+OmaToolContract\s+for\s+SasosTool(\d{3})\s*\{(.*?)"
    r"(?=\bpub\s+struct\s+SasosTool\d{3}\s*;|//\s*-+\s*\n//\s*4\.|\Z)",
    re.S,
)
INSERT = re.compile(
    rf"tools\.insert\(\s*({STRING})\s*,\s*Box::new\(SasosTool(\d{{3}})\)\s*\);"
)


def rust_string(value: str) -> str:
    return json.loads(value)


def field(body: str, name: str) -> str:
    match = re.search(rf"fn\s+{name}\s*\([^)]*\).*?\{{\s*({STRING})\s*\}}", body, re.S)
    if not match:
        raise ValueError(f"missing {name}")
    return rust_string(match.group(1))


def string_list(body: str, pattern: str) -> list[str]:
    match = re.search(pattern, body, re.S)
    if not match:
        raise ValueError(f"missing {pattern}")
    values = [rust_string(item.group()) for item in re.finditer(STRING, match.group(1))]
    if not values:
        raise ValueError("empty list")
    return values


def parse_catalog(source: str, expected_count: int = 555) -> list[dict]:
    catalog = []
    for number, body in IMPL.findall(source):
        clearance = re.search(
            r"fn\s+security_clearance\s*\([^)]*\).*?\{\s*SecurityClearance::(Ring[012](?:Kernel|Supervisor|Worker))\s*\}",
            body,
            re.S,
        )
        if not clearance:
            raise ValueError(f"invalid clearance for {number}")
        entry = {
            "number": int(number),
            "tool_id": field(body, "tool_id"),
            "name": field(body, "name"),
            "description": field(body, "description"),
            "required_fields": string_list(body, r"required_fields:\s*vec!\[(.*?)\]"),
            "connected_modules": string_list(body, r"fn\s+connected_modules.*?\{\s*&\[(.*?)\]"),
            "declared_clearance": clearance.group(1),
            "declared_runtime": field(body, "execution_runtime"),
            "status": "CATALOG_ONLY",
            "executable": False,
            "imandra_verified": False,
            "measured_latency_microseconds": None,
            "zero_copy_verified": False,
        }
        if not entry["tool_id"].startswith(f"SASOS_TOOL_{number}_"):
            raise ValueError(f"ID/number mismatch for {number}")
        catalog.append(entry)

    numbers = [entry["number"] for entry in catalog]
    if numbers != list(range(1, expected_count + 1)):
        raise ValueError("definitions must contain every number exactly once in order")
    ids = [entry["tool_id"] for entry in catalog]
    if len(set(ids)) != expected_count:
        raise ValueError("duplicate tool ID")
    insertions = [(rust_string(tool_id), int(number)) for tool_id, number in INSERT.findall(source)]
    if insertions != list(zip(ids, numbers)):
        raise ValueError("registry insertions differ from definitions")
    return catalog


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="complete Rust source provided by the catalog author")
    parser.add_argument("output", type=Path, help="JSON catalog output")
    args = parser.parse_args()
    catalog = parse_catalog(args.source.read_text(encoding="utf-8"))
    args.output.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Validated {len(catalog)} catalog-only entries: {args.output}")


if __name__ == "__main__":
    main()
