#!/usr/bin/env python3
"""Deterministic complete root README OCAF Roadmap: 195+64+10 = 269 numbered rows.

The 10 Agents Builder variants are original products under 0.50-0.55, NOT
extra stage IDs. A successful laboratory test is NOT production approval.
CI compares the rendered full section against README's very first section.
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


def render_section(inventory: dict, builders: dict) -> str:
    if (not isinstance(inventory,dict) or not isinstance(builders,dict)
        or len(inventory.get("main",[])) != 195
        or len(inventory.get("technical",[])) != 64
        or len(builders.get("products",[])) != 10
        or inventory.get("production_approved") is not False):
        raise ReadmeRoadmapError("incomplete / unverified source manifest")
    products=builders["products"]
    if len(set((p.get("platform"),p.get("mode")) for p in products)) != 10:
        raise ReadmeRoadmapError("missing Code or NoCode platform variant")
    items=[
        *(line(i+1,x,"main") for i,x in enumerate(inventory["main"])),
        *(line(i+196,x,"technical") for i,x in enumerate(inventory["technical"])),
        *(line(i+260,x,"product") for i,x in enumerate(products)),
    ]
    if len(items)!=269:
        raise ReadmeRoadmapError("not exactly 269 sequential numbered roadmap rows")
    parts=[
        BEGIN,
        "# OCAF — PEŁNA ROADMAPA OD POCZĄTKU, PUNKT PO PUNKCIE (1–269)",
        "",
        "**To jest pierwsza sekcja README.md i pełna lista statusów, nie tylko odsyłacz.** Każdy numer ma informację **ZROBIONE** albo **DO ZROBIENIA**. `ZROBIONE — ZAKRES IMPLEMENTACJI` oznacza kod i testy historyczne obecne w repo (NIE pełny odbiór ani produkcję); `ZROBIONE — TEST W OGRANICZONYM ZAKRESIE` oznacza wyłącznie potwierdzoną próbę laboratoryjną.",
        "",
        "**Zakres inwentaryzacji:** **1–195 = wszystkie historyczne i nowe etapy główne (0.00–1.94)**, **196–259 = 64 podetapy techniczne**, **260–269 = 10 pierwotnych produktów Agents Builder Code/NoCode**. Suma **269 kolejno numerowanych pozycji w README** = 259 punktów kamieni milowych/podetapów + 10 osobnych pozycji produktowych (nie są dodatkowymi numerami etapów). **33 repozytoria referencyjne nie są etapami.**",
        "",
        "**Ważne:** etapy historyczne bez dokumentów mają status DO ZROBIENIA (odtworzenie i weryfikacja). Prace zakończone administracyjnie, lecz niepotwierdzone technicznie, też mają status DO ZROBIENIA; opis statusu źródłowego wyjaśnia zakres. Nie uznajemy nieprzetestowanej funkcji za ukończoną. Dane synchronizuje automat walidacyjny z manifestów; nowe realne punkty dopisywane są dopiero po ostatnim ID **1.94**, a pierwotne, zagubione wymagania wracają pod starym ID.",
        "",
        "**Kanoniczny pełny dokument:** [ROADMAP.md](ROADMAP.md) · [lustrzana Roadmapa](docs/ROADMAP_FULL_ECOSYSTEM_2026.md) · [manifest 195+64](docs/roadmap/canonical_v11_inventory.json) · [dziesięć Agents Builder](docs/roadmap/agents_builder_products_v11.json) · [rejestr odzyskanych braków](docs/INVENTORY_LOST_REQUIREMENTS_2026-10-15.md) · [dziennik aktualizacji po każdym etapie](docs/roadmap/STAGE_CHANGELOG.md).",
        "",
        "## Punkty 1–195 — wszystkie etapy główne OCAF od 0.00 do 1.94",
        "",
        *items[:195],
        "",
        "## Punkty 196–259 — wszystkie 64 zidentyfikowane podetapy techniczne",
        "",
        *items[195:259],
        "",
        "## Punkty 260–269 — pierwotne Agents Builder: Cloud, Windows, macOS, Android, iOS × Code/NoCode",
        "",
        *items[259:],
        "",
        "### Reguła aktualizacji na zakończenie każdego etapu",
        "",
        "Przy **każdym** zakończonym etapie aktualizować w tym samym commicie: odpowiedni status pozycji powyżej, `ROADMAP.md`, jego identyczną kopię `docs/ROADMAP_FULL_ECOSYSTEM_2026.md`, maszynowy `docs/roadmap/canonical_v11_inventory.json`, `docs/roadmap/STAGE_CHANGELOG.md`, wyniki pozytywnych i negatywnych testów, commit i zakończony run GitHub Actions. Wszelkie nowe wymagania dostają wyłącznie **kolejny wolny identyfikator** (teraz 1.95), a odzyskane historyczne wymagania zachowują oryginalny. Lista 1–269 jest deterministycznie odtwarzana z manifestów, żeby zmiany statusów nie były ręcznie pomijane.",
        "",
        END,
        "",
        "",
    ]
    return "\n".join(parts)


def validate_readme(readme: str, inventory: dict, builders: dict) -> dict:
    if not isinstance(readme,str):
        raise ReadmeRoadmapError("README is not text")
    expected=render_section(inventory,builders)
    if not readme.startswith(expected) or readme.count(BEGIN)!=1 or readme.count(END)!=1:
        raise ReadmeRoadmapError("README does not BEGIN with full current source-derived sequential 269-line roadmap")
    rows=re.findall(r"^([0-9]+)\. \*\*\[([^]]+)\]\*\* \*\*",readme[:len(expected)],re.MULTILINE)
    if [int(i) for i,_ in rows]!=list(range(1,270)):
        raise ReadmeRoadmapError("README status list not numbered 1..269 with every ID represented")
    if not all(status in ("DO ZROBIENIA","ZROBIONE — TEST W OGRANICZONYM ZAKRESIE","ZROBIONE — ZAKRES IMPLEMENTACJI") for _,status in rows):
        raise ReadmeRoadmapError("at least one roadmap item lacks explicit completion status")
    return {
        "status":"OCAF_README_ROADMAP_1_TO_269_SOURCE_STATUS_PARITY_VERIFIED",
        "main_stage_items":195,
        "technical_substage_items":64,
        "original_agents_builder_variants":10,
        "numbered_items":269,
        "main_stages_marked_production_complete":0,
        "source_of_truth":"docs/roadmap/canonical_v11_inventory.json",
        "production_approved":False,
    }


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--repo-root",type=Path,default=Path("."))
    p.add_argument("--check",action="store_true")
    args=p.parse_args()
    root=args.repo_root
    try:
        inv=json.loads((root/"docs/roadmap/canonical_v11_inventory.json").read_text(encoding="utf-8"))
        builders=json.loads((root/"docs/roadmap/agents_builder_products_v11.json").read_text(encoding="utf-8"))
        readme_path=root/"README.md"
        current=readme_path.read_text(encoding="utf-8")
        if args.check:
            result=validate_readme(current,inv,builders)
        else:
            section=render_section(inv,builders)
            if current.startswith(BEGIN):
                if current.count(END)!=1:
                    raise ReadmeRoadmapError("existing generated README fence malformed")
                current=current.split(END,1)[1].lstrip("\n")
            readme_path.write_text(section+current,encoding="utf-8")
            result=validate_readme(readme_path.read_text(encoding="utf-8"),inv,builders)
    except (ValueError,OSError,TypeError,KeyError,UnicodeError) as exc:
        p.error(str(exc))
    print(json.dumps(result,sort_keys=True))
    return 0


if __name__=="__main__":
    raise SystemExit(main())
