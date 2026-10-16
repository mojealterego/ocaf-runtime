#!/usr/bin/env python3
"""Non-destructive validation of pinned README OCAF Roadmap (manifest-derived counts).

The 10 Agents Builder variants are original products under 0.50-0.55, NOT
extra stage IDs. A successful laboratory test is NOT production approval.
CI checks the protected first README section without overwriting it.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import re
import sys

# Implementation exists for these HISTORICAL REPOSITORY SCOPES in source/tests.
# This is NOT proof of full milestone sign-off, independent-host HA or production.
# Specific checked source/test paths are recorded in the accompanying audit.
HISTORICAL_IMPLEMENTED_SCOPES = frozenset(
    {f"0.{i:02d}" for i in (6,7,8,9,*range(11,36))}
)
ORIGINAL_EARLY_TITLES = {
    "0.06": "Durable local jobs (v0.6)",
    "0.07": "Authenticated capability coordinator (v0.7)",
    "0.08": "Distributed job orchestration (v0.8)",
    "0.09": "Local active/passive coordinator and transport preflight (v0.9)",
}

BEGIN = "<!-- OCAF_README_ROADMAP_BEGIN -->"
END = "<!-- OCAF_README_ROADMAP_END -->"


class ReadmeRoadmapError(ValueError):
    pass


def clean(value: str) -> str:
    if not isinstance(value, str):
        raise ReadmeRoadmapError("roadmap scope/id/title/status must be strings")
    return " ".join(value.replace("|", "/").split())


def mark(status: str, kind: str, row: dict | None = None) -> str:
    if (kind == "main" and isinstance(row,dict)
        and row.get("id") in HISTORICAL_IMPLEMENTED_SCOPES
        and status in ("UNSPECIFIED_IN_CURRENT_REPO", "HISTORICAL_NEEDS_VERIFICATION")):
        return "ZROBIONE — ZAKRES IMPLEMENTACJI"
    if kind == "product":
        return "DO ZROBIENIA"
    if kind == "technical" and "PASS" in status and "OPEN" not in status and "NO-GO" not in status:
        return "ZROBIONE — TEST W OGRANICZONYM ZAKRESIE"
    return "DO ZROBIENIA"


def line(num: int, row: dict, kind: str) -> str:
    if not isinstance(row, dict) or kind not in ("main", "technical", "product"):
        raise ReadmeRoadmapError("bad source roadmap row")
    status=mark(clean(row["status"]),kind,row)
    ident=clean(row["id"])
    title=clean(row["product"] if kind=="product" else ORIGINAL_EARLY_TITLES.get(row["id"],row["title"]))
    scope=clean(row["scope"])
    criterion=clean(row["real_acceptance"] if kind=="product" else row["acceptance"])
    extra=" — wymagania pierwotne etapów "+row["platform_stage"] if kind=="product" else ""
    if kind=="main" and row["id"] in HISTORICAL_IMPLEMENTED_SCOPES:
        extra+=" — **Kod i testy istnieją w repo; pełny odbiór etapu i produkcji: NIE**"
        extra+="; dowody: [audyt](docs/HISTORICAL_IMPLEMENTATION_AUDIT_2026-10-15.md)"
    return (
        f"{num}. **[{status}]** **{ident} — {title}**{extra}. {scope} "
        f"**Kryterium zakończenia:** {criterion}. **Stan źródłowy:** `{clean(row['status'])}`."
    )


def render_pinned_panel(builders:dict)->list[str]:
    """First GitHub-rendered README element: visible bordered roadmap STATUS table.

    This is not a GitHub special pinned feature or a collapsed <details>.
    A two-column Markdown table renders immediately under the repository files.
    Always derive Agents Builder statuses from the canonical 10-product manifest.
    """
    if not isinstance(builders,dict) or not isinstance(builders.get("products"),list):
        raise ReadmeRoadmapError("missing canonical Agents Builder product manifest")
    if len(builders["products"])!=10 or not all(
            isinstance(p,dict) for p in builders["products"]):
        raise ReadmeRoadmapError("exactly ten canonical Builder products required")
    pairs={(p.get("platform"),p.get("mode")):p for p in builders["products"]}
    platforms=(("Cloud","Chmurowy"),("Windows","Desktopowy Windows"),
               ("macOS","Desktopowy macOS"),("Android","Android"),("iOS","iOS"))
    if len(pairs)!=10 or any((platform,mode) not in pairs
                             for platform,_ in platforms for mode in ("Code","NoCode")):
        raise ReadmeRoadmapError("top panel must show all five Code and NoCode builder pairs")
    status_labels={"NOT_IMPLEMENTED":"[DO ZROBIENIA]",
                   "PARTIAL":"[W TRAKCIE]",
                   "IMPLEMENTED_VERIFIED":"[ZROBIONE — ZWERYFIKOWANY ZAKRES]"}
    rows=[
        "| 📌 **ROADMAPA OCAF — PRZYPIĘTA NA GÓRZE REPOZYTORIUM** | **STATUS / SZYBKI DOSTĘP** |",
        "| :--- | :--- |",
        "| **[OTWÓRZ PEŁNĄ ROADMAPĘ](ROADMAP.md)** | **195 etapów + 64 podetapy + 10 produktów Agents Builder = 269 pozycji** |",
        "| **[Pełna lista statusów od 0.00 do 1.94](ROADMAP.md)** | **ZROBIONE / W TRAKCIE / DO ZROBIENIA — szczegóły bezpośrednio pod ramką** |",
    ]
    for platform,label in platforms:
        code=pairs[platform,"Code"]
        nocode=pairs[platform,"NoCode"]
        if (code.get("platform_stage")!=nocode.get("platform_stage")
            or code.get("status") not in status_labels
            or nocode.get("status") not in status_labels):
            raise ReadmeRoadmapError("unverified Agents Builder pair status or original stage")
        rows.append(
            f"| **Agents Builder {label} — Code + NoCode (etap {code['platform_stage']})** "
            f"| **Code: {status_labels[code['status']]} · NoCode: {status_labels[nocode['status']]}** |"
        )
    rows.extend((
        "| **PRIORYTET: 0.39 / 0.40 / 0.41** | **WASI w Firecracker: W TRAKCIE; PostgreSQL HA między hostami: DO ZROBIENIA; testy niezawodności: W TRAKCIE** |",
        "| **REGUŁA STAŁEJ AKTUALIZACJI** | **Po każdym etapie: status, realny test/CI, commit, artefakt, changelog; nowe punkty na końcu** |",
        "| **[Inwentaryzacja braków](docs/INVENTORY_LOST_REQUIREMENTS_2026-10-15.md)** | **[10 pierwotnych Agents Builder](docs/AGENTS_BUILDER_CODE_NOCODE_5_PLATFORMS.md) · [Dziennik zmian etapów](docs/roadmap/STAGE_CHANGELOG.md)** |",
        "",
    ))
    return rows


def validate_readme(readme: str, inventory: dict, builders: dict) -> dict:
    """Fail closed if the pinned, full historical README registry is incomplete."""
    if not isinstance(readme, str) or not readme.startswith(BEGIN):
        raise ReadmeRoadmapError("Roadmap must remain at the first byte of README")
    if readme.count(BEGIN) != 1 or readme.count(END) != 1:
        raise ReadmeRoadmapError("README roadmap fence is missing or duplicated")
    pinned = readme.split(END, 1)[0]
    main = inventory.get("main", [])
    technical = inventory.get("technical", [])
    products = builders.get("products", [])
    if len(main) != inventory.get("main_count") or len(technical) != inventory.get("technical_count"):
        raise ReadmeRoadmapError("Manifest count mismatch")
    if len(products) != 10 or len(set((p.get("platform"), p.get("mode")) for p in products)) != 10:
        raise ReadmeRoadmapError("Missing original Agents Builder Code/NoCode products")
    if inventory.get("production_approved") is not False:
        raise ReadmeRoadmapError("Unverified production approval")
    if "UWAGA CZYTAĆ" not in pinned or "ZASADA STAŁEJ ROZBUDOWY ROADMAPY" not in pinned or "ZABRONIONA JEST SAMOWOLKA" not in pinned:
        raise ReadmeRoadmapError("Pinned warning or preservation rule missing")
    ids = [x["id"] for x in main + technical]
    for stage_id in ids:
        if not re.search(r"^\\| \\*\\*" + re.escape(stage_id) + r"\\*\\* \\|", pinned, re.MULTILINE):
            raise ReadmeRoadmapError(f"Missing historical stage: {stage_id}")
    for product in products:
        platform, mode = product["platform"], product["mode"]
        if platform.lower() not in pinned.lower() or mode not in pinned:
            raise ReadmeRoadmapError(f"Missing product: {platform} {mode}")
    total = len(main) + len(technical) + len(products)
    if f"1–{total}" not in pinned:
        raise ReadmeRoadmapError("Pinned roadmap total is stale")
    return {
        "status": "PINNED_ROADMAP_MANIFEST_IDS_VERIFIED",
        "main_stage_items": len(main),
        "technical_substage_items": len(technical),
        "original_agents_builder_variants": len(products),
        "numbered_items": total,
        "production_approved": False,
    }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--repo-root", type=Path, default=Path("."))
    p.add_argument("--check", action="store_true")
    args = p.parse_args()
    root = args.repo_root
    try:
        inv = json.loads((root / "docs/roadmap/canonical_v11_inventory.json").read_text(encoding="utf-8"))
        builders = json.loads((root / "docs/roadmap/agents_builder_products_v11.json").read_text(encoding="utf-8"))
        readme = (root / "README.md").read_text(encoding="utf-8")
        result = validate_readme(readme, inv, builders)
    except (ValueError, OSError, TypeError, KeyError, UnicodeError) as exc:
        p.error(str(exc))
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
