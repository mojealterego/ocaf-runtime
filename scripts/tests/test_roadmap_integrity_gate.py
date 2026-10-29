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
