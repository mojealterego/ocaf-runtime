import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/roadmap_integrity_gate.py"


def test_current_counts_and_order():
    result = subprocess.run([sys.executable, str(SCRIPT), "--repo", str(ROOT)], check=True, capture_output=True, text=True)
    data = json.loads(result.stdout)
    assert data["declared_main"] == 334
    assert data["detailed_main"] == 334
    assert data["append_only"] is True


def test_snapshot_gate():
    snapshot = Path("/tmp/ocaf-recovery-builds-20261021")
    if not (snapshot / "source.sha256").exists():
        return
    result = subprocess.run([sys.executable, str(SCRIPT), "--repo", str(ROOT), "--snapshot", str(snapshot)], check=True, capture_output=True, text=True)
    assert json.loads(result.stdout)["snapshot_checked"] is True


def test_technical_and_reference_records_are_structurally_validated(tmp_path):
    """Mutation tests: the gate must refuse missing, renamed and reordered IDs."""
    import copy

    import pytest

    from scripts.roadmap_integrity_gate import IntegrityError, _validate_manifest

    source = json.loads((ROOT / "docs/roadmap/canonical_v11_inventory.json").read_text(encoding="utf-8"))
    directory = tmp_path / "docs" / "roadmap"
    directory.mkdir(parents=True)
    target = directory / "canonical_v11_inventory.json"

    def check(mutator):
        manifest = copy.deepcopy(source)
        mutator(manifest)
        target.write_text(json.dumps(manifest, ensure_ascii=False), encoding="utf-8")
        with pytest.raises(IntegrityError):
            _validate_manifest(tmp_path)

    target.write_text(json.dumps(source, ensure_ascii=False), encoding="utf-8")
    _validate_manifest(tmp_path)

    check(lambda m: m["technical"].pop())
    check(lambda m: m["technical"][1].update(id=m["technical"][0]["id"]))
    check(lambda m: m["technical"][1:3].reverse())
    check(lambda m: m["technical"][0].update(title=" "))
    check(lambda m: m["reference_repositories"].pop())
    check(lambda m: m["reference_repositories"][0].update(
        name=m["reference_repositories"][1]["name"].upper()
    ))
    check(lambda m: m["reference_repositories"][0].update(name="../escape"))
