#!/usr/bin/env python3
"""Deterministic, SHA-pinned OCAF Agents Builder and full-source ZIP bundles.

All inputs come exclusively from Git-tracked regular files at HEAD.
No secrets, build cache, untracked files or binary weight files are copied.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
from pathlib import Path, PurePosixPath
import stat
import subprocess
import sys
import tempfile
import zipfile

FIXED_TIME = (1980, 1, 1, 0, 0, 0)
MAX_SOURCE_BYTES = 30 * 1024 * 1024
MAX_ENTRY_BYTES = 5 * 1024 * 1024
MAX_FILES = 10000

EXACT = frozenset({
    "README.md", "ROADMAP.md", "Cargo.toml", "Cargo.lock",
    "src/lib.rs", "src/model_router.rs",
    "src/stage044_oauth.rs", "src/stage044_provider_refresh.rs",
    "docs/AGENTS_BUILDER_CODE_NOCODE_5_PLATFORMS.md",
    "docs/roadmap/agents_builder_products_v11.json",
    "docs/roadmap/STAGE_CHANGELOG.md",
    "scripts/package_agents_builder.py",
    "scripts/tests/test_agents_builder_local_runner.py",
    "scripts/tests/test_builder_native_desktop.py",
    "scripts/tests/test_builder_mobile_native.py",
    "scripts/tests/test_package_agents_builder.py",
})
PREFIXES = (
    "apps/agents-builder-web/",
    "apps/agents-builder-desktop/",
    "apps/agents-builder-android/",
    "apps/agents-builder-ios/",
    "docs/audits/STAGE050_BUILDER",
    "docs/audits/STAGE051_BUILDER",
    "docs/audits/STAGE052_BUILDER",
    "docs/audits/STAGE052_UNIFIED_UI",
)
WORKFLOWS = frozenset({
    ".github/workflows/stage050-builder-core.yml",
    ".github/workflows/stage051-builder-web.yml",
    ".github/workflows/stage052-builder-cloud.yml",
    ".github/workflows/agents-builder-source-bundle.yml",
    ".github/workflows/agents-builder-native-desktop.yml",
    ".github/workflows/agents-builder-native-mobile.yml",
})


def publish_release_directory(staged: Path, destination: Path) -> None:
    """Publish a complete release atomically, refusing an existing destination.

    Linux uses renameat2(RENAME_NOREPLACE); macOS uses renamex_np(RENAME_EXCL).
    Windows rename fails if the target exists. Unknown platforms fail closed.
    Both paths must be siblings on the same filesystem.
    """
    import ctypes
    import errno

    if staged.parent != destination.parent:
        raise ValueError("release staging and destination must be siblings")
    if sys.platform.startswith("linux"):
        libc = ctypes.CDLL(None, use_errno=True)
        renameat2 = getattr(libc, "renameat2", None)
        if renameat2 is None:
            raise OSError(errno.ENOTSUP, "atomic no-replace rename unavailable")
        renameat2.argtypes = [
            ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p,
            ctypes.c_uint,
        ]
        renameat2.restype = ctypes.c_int
        result = renameat2(
            -100, os.fsencode(staged), -100, os.fsencode(destination), 1,
        )
        if result != 0:
            code = ctypes.get_errno()
            if code == errno.EEXIST:
                raise FileExistsError(code, os.strerror(code), str(destination))
            raise OSError(code, os.strerror(code), str(destination))
    elif sys.platform == "darwin":
        libc = ctypes.CDLL(None, use_errno=True)
        renamex_np = getattr(libc, "renamex_np", None)
        if renamex_np is None:
            raise OSError(errno.ENOTSUP, "atomic no-replace rename unavailable")
        renamex_np.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint]
        renamex_np.restype = ctypes.c_int
        if renamex_np(os.fsencode(staged), os.fsencode(destination), 0x00000004) != 0:
            code = ctypes.get_errno()
            if code == errno.EEXIST:
                raise FileExistsError(code, os.strerror(code), str(destination))
            raise OSError(code, os.strerror(code), str(destination))
    elif os.name == "nt":
        # MoveFileEx without MOVEFILE_REPLACE_EXISTING (Python os.rename).
        os.rename(staged, destination)
    else:
        raise OSError(errno.ENOTSUP, "atomic no-replace rename unavailable")


def git(repo: Path, *args: str) -> bytes:
    return subprocess.check_output(
        ["git", "-C", str(repo), *args], stderr=subprocess.DEVNULL
    )


def current_sha(repo: Path) -> str:
    sha = git(repo, "rev-parse", "HEAD").decode("ascii").strip()
    if len(sha) != 40 or any(c not in "0123456789abcdef" for c in sha):
        raise ValueError("invalid git commit SHA")
    return sha


def select_builder(path: str) -> bool:
    return (
        path in EXACT
        or path in WORKFLOWS
        or any(path.startswith(prefix) for prefix in PREFIXES)
        or (path.startswith("src/builder_") and path.endswith(".rs"))
        or (path.startswith("tests/builder_") and path.endswith(".rs"))
    )


def tracked_files(repo: Path) -> list[str]:
    paths = [os.fsdecode(raw) for raw in git(repo, "ls-files", "-z").split(b"\0") if raw]
    if len(paths) > MAX_FILES:
        raise ValueError("too many tracked files")
    if len(paths) != len(set(paths)):
        raise ValueError("duplicate tracked files")
    for path in paths:
        p = PurePosixPath(path)
        if (
            not path
            or path.startswith("/")
            or "\\" in path
            or any(part in ("", ".", "..") for part in path.split("/"))
            or p.as_posix() != path
        ):
            raise ValueError(f"unsafe Git path: {path!r}")
    return sorted(paths)


def archive_bytes(repo: Path, path: str) -> bytes:
    # Reject path traversal even for direct callers, not only tracked_files().
    parts = path.split("/")
    if (
        not path
        or path.startswith("/")
        or "\\" in path
        or any(part in ("", ".", "..") for part in parts)
        or PurePosixPath(path).as_posix() != path
    ):
        raise ValueError("unsafe bundle entry path")
    # Traverse by directory descriptors: path names cannot redirect the
    # final read after an earlier lstat check (TOCTOU). Never follow symlinks.
    if not hasattr(os, "O_NOFOLLOW") or not hasattr(os, "O_DIRECTORY"):
        raise ValueError("secure descriptor-relative opening unavailable")
    descriptors = []
    try:
        root_fd = os.open(repo, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        descriptors.append(root_fd)
        for part in parts[:-1]:
            directory_fd = os.open(
                part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                dir_fd=descriptors[-1],
            )
            descriptors.append(directory_fd)
        file_fd = os.open(
            parts[-1], os.O_RDONLY | os.O_NOFOLLOW | getattr(os, "O_NONBLOCK", 0),
            dir_fd=descriptors[-1],
        )
        descriptors.append(file_fd)
        metadata = os.fstat(file_fd)
        if not stat.S_ISREG(metadata.st_mode):
            raise ValueError(f"refusing non-regular tracked file: {path}")
        if metadata.st_size > MAX_ENTRY_BYTES:
            raise ValueError(f"oversized tracked file: {path}")
        with os.fdopen(os.dup(file_fd), "rb") as source:
            content = source.read(MAX_ENTRY_BYTES + 1)
        if len(content) > MAX_ENTRY_BYTES:
            raise ValueError(f"oversized tracked file: {path}")
        return content
    except OSError as error:
        raise ValueError(f"secure source open rejected: {path}") from error
    finally:
        for descriptor in reversed(descriptors):
            os.close(descriptor)


def write_bundle(repo: Path, dst: Path, sha: str, files: list[str], kind: str) -> dict:
    if not files:
        raise ValueError("empty source bundle")
    label = f"ocaf-{kind}-{sha[:12]}"
    manifest = {
        "format": "ocaf-source-bundle-v1",
        "source_commit": sha,
        "scope": kind,
        "status": "SOURCE_SNAPSHOT_NOT_PRODUCTION_APPROVAL",
        "files": [],
    }
    dst.parent.mkdir(parents=True, exist_ok=True)
    # Stage in the destination filesystem; verification must succeed before
    # the final name becomes visible. A failed attempt preserves an old ZIP.
    fd, staged_name = tempfile.mkstemp(prefix=f".{dst.name}.", suffix=".tmp",
                                     dir=dst.parent)
    staged = Path(staged_name)
    try:
        os.close(fd)
        total = 0
        with zipfile.ZipFile(staged, "w", compression=zipfile.ZIP_DEFLATED,
                             compresslevel=9, allowZip64=True) as archive:
            for path in files:
                content = archive_bytes(repo, path)
                total += len(content)
                if total > MAX_SOURCE_BYTES:
                    raise ValueError("source snapshot exceeds 30 MiB safety limit")
                manifest["files"].append({
                    "path": path,
                    "size": len(content),
                    "sha256": hashlib.sha256(content).hexdigest(),
                })
                info = zipfile.ZipInfo(f"{label}/{path}", FIXED_TIME)
                info.compress_type = zipfile.ZIP_DEFLATED
                info.external_attr = (0o100644 << 16)
                archive.writestr(info, content, compress_type=zipfile.ZIP_DEFLATED,
                                 compresslevel=9)
            raw = (json.dumps(manifest, indent=2, ensure_ascii=False, sort_keys=True)
                   + "\n").encode("utf-8")
            info = zipfile.ZipInfo(f"{label}/RELEASE_MANIFEST.json", FIXED_TIME)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o100644 << 16)
            archive.writestr(info, raw, compress_type=zipfile.ZIP_DEFLATED,
                             compresslevel=9)
        verify_bundle(staged, sha, files, repo)
        # Reject a concurrent Git commit after verification but before publish.
        # The source manifest must describe the repository's current HEAD.
        if current_sha(repo) != sha:
            raise ValueError("HEAD changed during source bundle packaging")
        require_clean_tree(repo)
        with staged.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
            size = os.fstat(source.fileno()).st_size
        os.replace(staged, dst)
    finally:
        staged.unlink(missing_ok=True)
    return {"name": dst.name, "bytes": size, "sha256": digest,
            "file_count": len(files)}


def verify_bundle(archive_path: Path, sha: str, files: list[str], repo: Path) -> None:
    with zipfile.ZipFile(archive_path) as z:
        if z.testzip() is not None:
            raise ValueError("ZIP CRC failure")
        names = z.namelist()
        if len(names) != len(files) + 1 or len(set(names)) != len(names):
            raise ValueError("ZIP missing or duplicate file")
        manifests = [name for name in names if name.endswith("/RELEASE_MANIFEST.json")]
        if len(manifests) != 1:
            raise ValueError("missing manifest")
        prefix = manifests[0].removesuffix("RELEASE_MANIFEST.json")
        manifest = json.loads(z.read(manifests[0]))
        if manifest["source_commit"] != sha:
            raise ValueError("source SHA mismatch")
        if [item["path"] for item in manifest["files"]] != files:
            raise ValueError("manifest does not cover every tracked file")
        for item in manifest["files"]:
            path = item["path"]
            if hashlib.sha256(z.read(prefix + path)).hexdigest() != item["sha256"]:
                raise ValueError("ZIP file hash mismatch: " + path)
            committed = git(repo, "show", f"{sha}:{path}")
            if z.read(prefix + path) != committed:
                raise ValueError("ZIP differs from committed source at Git HEAD: " + path)
            if committed != archive_bytes(repo, path):
                raise ValueError("ZIP differs from checked-out source: " + path)


def require_clean_tree(repo: Path) -> None:
    # Never sign a dirty worktree using a Git commit identifier.
    if git(repo, "status", "--porcelain=v1", "--untracked-files=no").strip():
        raise ValueError("tracked Git worktree is dirty")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--out", type=Path, default=Path("dist"))
    parser.add_argument("--expected-sha", default="")
    args = parser.parse_args()
    repo = args.repo.resolve(strict=True)
    sha = current_sha(repo)
    require_clean_tree(repo)
    if args.expected_sha and sha != args.expected_sha:
        raise ValueError(f"HEAD {sha} != expected SHA {args.expected_sha}")
    paths = tracked_files(repo)
    selected = [p for p in paths if select_builder(p)]
    required = ("apps/agents-builder-web/index.html", "apps/agents-builder-web/app.mjs",
                "apps/agents-builder-web/local_runner.py",
                "apps/agents-builder-web/runner-client.mjs",
                "scripts/tests/test_agents_builder_local_runner.py",
                "apps/agents-builder-web/models.mjs", "apps/agents-builder-web/i18n.mjs",
                "apps/agents-builder-web/credentials.mjs", "src/builder_core.rs",
                "src/builder_cloud.rs", "src/builder_http.rs",
                "src/builder_store.rs", "tests/builder_web_contract.rs",
                "scripts/package_agents_builder.py")
    missing = [path for path in required if path not in selected]
    if missing:
        raise ValueError("required builder files missing: " + ", ".join(missing))
    # Publish the entire four-file release as one directory entry. Never
    # expose a valid-looking partial set if the second ZIP or metadata fails.
    # An existing destination is deliberately immutable: no non-atomic
    # replacement of a populated directory and no silent overwrite.
    out = args.out
    out.parent.mkdir(parents=True, exist_ok=True)
    if out.exists() or out.is_symlink():
        raise FileExistsError(f"release destination already exists: {out}")
    staged = Path(tempfile.mkdtemp(prefix=f".{out.name}.", suffix=".tmp",
                                   dir=out.parent))
    try:
        results = [
            write_bundle(repo, staged / f"ocaf-agents-builder-{sha[:12]}.zip",
                         sha, selected, "agents-builder"),
            write_bundle(repo, staged / f"ocaf-runtime-full-source-{sha[:12]}.zip",
                         sha, paths, "runtime-full-source"),
        ]
        sums = "".join(f'{item["sha256"]}  {item["name"]}\n' for item in results)
        (staged / "SHA256SUMS").write_text(sums, encoding="ascii")
        (staged / "bundle-report.json").write_text(
            json.dumps({"sha": sha, "artifacts": results}, indent=2) + "\n",
            encoding="utf-8",
        )
        if current_sha(repo) != sha:
            raise ValueError("HEAD changed during release assembly")
        require_clean_tree(repo)
        # On the same filesystem, directory rename publishes all artifacts
        # together. Do not replace an existing release or follow a symlink.
        if out.exists() or out.is_symlink():
            raise FileExistsError(f"release destination appeared: {out}")
        publish_release_directory(staged, out)
    finally:
        if staged.exists():
            shutil.rmtree(staged)
    print(json.dumps({"sha": sha, "artifacts": results}, indent=2))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.CalledProcessError,
            zipfile.BadZipFile) as error:
        print(f"packaging rejected: {error}", file=sys.stderr)
        sys.exit(1)
