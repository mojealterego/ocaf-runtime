#!/usr/bin/env bash
# Reproducible scoped checks. Never deploy or create paid infrastructure.
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
export PYTHONDONTWRITEBYTECODE=1
export LC_ALL=C
if [[ $# -ne 1 || ! "$1" =~ ^[0-9a-f]{40}$ ]]; then
    printf 'usage: %s <expected-git-commit>\n' "$0" >&2
    exit 2
fi
actual=$(git rev-parse HEAD)
if [[ "$actual" != "$1" ]]; then
    printf 'unexpected HEAD %s\n' "$actual" >&2
    exit 1
fi
printf 'source_commit=%s\n' "$actual"
rustc --version
cargo --version
python3 --version
node --version
openssl version
rustfmt --edition 2021 --check src/bin/ocaf-builder-cloud-bridge.rs tests/builder_cloud_full_stack.rs
cargo build --locked --no-default-features --bin ocaf-builder-cloud-bridge
cargo test --locked --no-default-features \
    --bin ocaf-builder-cloud-bridge \
    --test builder_cloud_full_stack --test builder_cloud \
    --test builder_store --test builder_core --test builder_web_contract
python3 -m unittest discover -s scripts/tests -p 'test_builder_cloud_*.py' -v
python3 -m unittest discover -s scripts/tests -p test_builder_bundle_platform_coverage.py -v
python3 -m unittest discover -s scripts/tests -p test_package_agents_builder.py -v
node --test apps/agents-builder-web/tests/*.test.mjs
# Report existing library diagnostics as well; never claim repo-wide strict Clippy.
scratch=$(mktemp)
trap 'rm -f -- "$scratch"' EXIT
cargo clippy --locked --no-default-features --bin ocaf-builder-cloud-bridge \
    --test builder_cloud_full_stack --message-format=json > "$scratch"
python3 - "$scratch" <<'PY'
import json
import sys
from pathlib import PurePosixPath
new = {'src/bin/ocaf-builder-cloud-bridge.rs', 'tests/builder_cloud_full_stack.rs'}
other = local = 0
with open(sys.argv[1], encoding='utf-8') as stream:
    for line in stream:
        message = json.loads(line)
        if message.get('reason') != 'compiler-message':
            continue
        diagnostic = message['message']
        if diagnostic['level'] not in ('warning', 'error'):
            continue
        paths = [str(PurePosixPath(s['file_name'])) for s in diagnostic.get('spans', []) if s['is_primary']]
        if not paths:
            continue
        if any(path in new or any(path.endswith('/'+n) for n in new) for path in paths):
            local += 1
        else:
            other += 1
        print(diagnostic.get('rendered', diagnostic.get('message', 'diagnostic')), file=sys.stderr)
print(f'clippy_new_source_diagnostics={local} clippy_other_primary_diagnostics={other}')
if local:
    raise SystemExit(1)
PY
sha256sum target/debug/ocaf-builder-cloud-bridge
printf 'SCOPED_BUILDER_CLOUD_TESTS_PASS (not release approval)\n'
