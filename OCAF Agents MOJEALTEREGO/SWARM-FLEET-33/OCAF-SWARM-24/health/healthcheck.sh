#!/usr/bin/env sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
command -v python3 >/dev/null 2>&1 || exit 2
python3 - "$ROOT/manifest.json" <<'PY2'
import json,sys
d=json.load(open(sys.argv[1]))
assert d["swarm_id"] == "OCAF-SWARM-24"
assert d["status"] == "STRUCTURED_NOT_DEPLOYED"
assert d["health"]["active_admission"] == "DENY"
assert d["health"]["ostc_enabled"] is False
assert d["member_count"] == len(d["members"])
print("OCAF-SWARM-24: manifest-health PASS; active=DENY; ostc=DISABLED")
PY2
