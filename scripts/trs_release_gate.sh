#!/usr/bin/env bash
# OCAF TRS release gate. Runs locally/on a self-hosted machine; GitHub Actions not required.
set -Eeuo pipefail
umask 077
export CARGO_TERM_COLOR=never
export RUST_BACKTRACE=1
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
OUT="${OCAF_EVIDENCE_DIR:-$ROOT/target/trs-release-evidence}"
mkdir -p "$OUT"
LOG="$OUT/verification.log"
exec > >(tee "$LOG") 2>&1
printf 'OCAF TRS verification started: %s\n' "$(date -u +%FT%TZ)"
# Never certify a source tree different from the commit packaged by git archive.
if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
  echo 'BLOCKED: tracked working tree is dirty; commit or revert before release' >&2
  exit 2
fi
git rev-parse HEAD | tee "$OUT/commit.txt"
command -v cargo >/dev/null || { echo 'BLOCKED: cargo missing' >&2; exit 127; }
command -v rustc >/dev/null || { echo 'BLOCKED: rustc missing' >&2; exit 127; }
cargo --version
rustc --version
# Fail closed: any failed step aborts packaging.
cargo fmt --all -- --check
cargo test --workspace --no-default-features --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --release --locked
cargo test --locked --test trs_dtn_sqlite_concurrent_claim
cargo test --locked --test trs_dtn_sqlite_e2e_recovery
cargo test --locked --test trs_dtn_sqlite_total_bytes
cargo test --locked --test trs_dtn_sqlite_oversized_blob
# Publish one ZIP with source, compiled release executables and verification evidence.
# A source-only tarball is not a distributable release.
python3 --version
python3 - "$ROOT" "$OUT" "$(git rev-parse HEAD)" <<'PY'
import hashlib
import io
import json
import pathlib
import subprocess
import sys
import zipfile

root = pathlib.Path(sys.argv[1]).resolve()
out = pathlib.Path(sys.argv[2]).resolve()
sha = sys.argv[3]
archive = out / f"ocaf-trs-{sha[:12]}.zip"
source = subprocess.run(
    ["git", "archive", "--format=tar", sha],
    cwd=root, check=True, stdout=subprocess.PIPE,
).stdout
release = root / "target" / "release"
# Include executable targets that cargo built; do not guess platform extensions.
executables = []
for name in (
    "ocaf-cluster-health", "ocaf-omas-server", "ocaf-omas-mtls-server",
    "ocaf-omat-server",
):
    for suffix in ("", ".exe"):
        candidate = release / (name + suffix)
        if candidate.is_file():
            executables.append(candidate)
if not executables:
    raise SystemExit("BLOCKED: no verified release executables found")
manifest = {
    "commit": sha,
    "source_sha256": hashlib.sha256(source).hexdigest(),
    "executables": {},
    "verification": "All preceding release-gate commands completed successfully",
}
for binary in executables:
    manifest["executables"][binary.name] = hashlib.sha256(binary.read_bytes()).hexdigest()
with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as z:
    z.writestr("source/ocaf-runtime.tar", source)
    for binary in executables:
        z.write(binary, f"bin/{binary.name}")
    for name in ("commit.txt", "verification.log"):
        item = out / name
        if item.exists():
            z.write(item, f"evidence/{name}")
    z.writestr("MANIFEST.json", json.dumps(manifest, indent=2, sort_keys=True) + "\n")
with zipfile.ZipFile(archive) as z:
    bad = z.testzip()
    if bad:
        raise SystemExit(f"BLOCKED: corrupt archive member {bad}")
digest = hashlib.sha256(archive.read_bytes()).hexdigest()
(out / (archive.name + ".sha256")).write_text(f"{digest}  {archive.name}\n")
(out / "STATUS").write_text("PASS\n")
print(f"Release gate PASS: {archive}")
PY
