#!/usr/bin/env python3
"""Exercise one source-bound release CLI against disposable PostgreSQL state."""
import argparse
import json
import os
from pathlib import Path

from scripts.stage042_rollback_drill import database_probe
from scripts.verify_release_bundle import BundleError


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--expected-pending", type=int, required=True)
    args = parser.parse_args()
    try:
        report = database_probe(args.bundle, args.expected_pending,
                                os.environ.get("OCAF_TEST_PG_DSN", ""))
    except BundleError as error:
        print(json.dumps({"status": "FAIL", "reason": str(error)}))
        return 1
    print(json.dumps({"status": "EPHEMERAL_PG_STATE_PASS", **report}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
