#!/usr/bin/env python3
"""Exercise one source-bound release CLI against disposable PostgreSQL state."""
import argparse
import json
import os
from pathlib import Path

from scripts.stage042_rollback_drill import database_probe, revoked_database_probe
from scripts.verify_release_bundle import BundleError


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path, required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--expected-pending", type=int)
    mode.add_argument("--expect-revoked", action="store_true")
    args = parser.parse_args()
    try:
        dsn = os.environ.get("OCAF_TEST_PG_DSN", "")
        report = (revoked_database_probe(args.bundle, dsn) if args.expect_revoked
                  else database_probe(args.bundle, args.expected_pending, dsn))
    except BundleError as error:
        print(json.dumps({"status": "FAIL", "reason": str(error)}))
        return 1
    print(json.dumps({"status": "EPHEMERAL_PG_REVOKE_PASS" if args.expect_revoked
                      else "EPHEMERAL_PG_STATE_PASS", **report}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
