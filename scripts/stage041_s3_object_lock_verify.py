#!/usr/bin/env python3
"""Verify four pinned encrypted Stage 0.41 bundle versions in S3 Object Lock.

This checks an offsite copy. It does not prove independent KMS, a durable-host
restore, or completion of Stage 0.41. AWS CLI credentials come from the usual
operator environment; no credentials or shell command are read from inventory.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import tempfile


MEMBER_LIMITS = {
    "source.json": 128 * 1024,
    "manifest.json": 4096,
    "envelope/manifest.json": 4096,
    "envelope/snapshot.aesgcm": 64 * 1024 * 1024 + 16,
}
INVENTORY_FIELDS = frozenset({"schema_version", "bucket", "expected_bucket_owner",
                              "key_prefix", "objects"})
OBJECT_FIELDS = frozenset({"name", "key", "version_id", "sha256", "size_bytes"})
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
BUCKET = re.compile(r"[a-z0-9][a-z0-9.-]{1,61}[a-z0-9]\Z")
PREFIX_SEGMENT = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]*\Z")
REGION = re.compile(r"[a-z]{2}(?:-[a-z0-9]+)+-[0-9]+\Z")
OWNER = re.compile(r"[0-9]{12}\Z")


class OffsiteVerificationError(ValueError):
    """An offsite copy or its local inventory did not satisfy the policy."""


def _unique(pairs: list[tuple[str, object]]) -> dict:
    record = {}
    for name, value in pairs:
        if name in record:
            raise OffsiteVerificationError("duplicate JSON field")
        record[name] = value
    return record


def _json(raw: bytes | str) -> dict:
    try:
        record = json.loads(raw, object_pairs_hook=_unique,
                            parse_constant=lambda _: (_ for _ in ()).throw(
                                OffsiteVerificationError("non-finite JSON number")))
    except (TypeError, ValueError, UnicodeError) as exc:
        raise OffsiteVerificationError("invalid JSON document") from exc
    if not isinstance(record, dict):
        raise OffsiteVerificationError("JSON object required")
    return record


def _digest_regular(path: Path, limit: int) -> tuple[str, int]:
    """Hash a single regular inode, rejecting links and changes during read."""
    if path.is_symlink():
        raise OffsiteVerificationError("symlinked bundle member or inventory")
    try:
        fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    except OSError as exc:
        raise OffsiteVerificationError("bundle member or inventory unavailable") from exc
    try:
        before = os.fstat(fd)
        if (not stat.S_ISREG(before.st_mode) or before.st_nlink != 1
                or not 0 < before.st_size <= limit):
            raise OffsiteVerificationError("unsafe or oversized regular file")
        digest = hashlib.sha256()
        size = 0
        while True:
            part = os.read(fd, min(1024 * 1024, limit + 1 - size))
            if not part:
                break
            digest.update(part)
            size += len(part)
            if size > limit:
                raise OffsiteVerificationError("oversized regular file")
        after = os.fstat(fd)
        if (size != before.st_size or before.st_ino != after.st_ino
                or before.st_size != after.st_size
                or before.st_mtime_ns != after.st_mtime_ns):
            raise OffsiteVerificationError("file changed while hashing")
        return digest.hexdigest(), size
    finally:
        os.close(fd)


def _read_small_regular(path: Path, limit: int) -> bytes:
    if path.is_symlink():
        raise OffsiteVerificationError("symlinked inventory")
    try:
        fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    except OSError as exc:
        raise OffsiteVerificationError("inventory unavailable") from exc
    try:
        before = os.fstat(fd)
        if (not stat.S_ISREG(before.st_mode) or before.st_nlink != 1
                or not 0 < before.st_size <= limit):
            raise OffsiteVerificationError("unsafe inventory inode")
        raw = os.read(fd, limit + 1)
        after = os.fstat(fd)
        if (len(raw) != before.st_size or before.st_size != after.st_size
                or before.st_ino != after.st_ino
                or before.st_mtime_ns != after.st_mtime_ns):
            raise OffsiteVerificationError("inventory changed while reading")
        return raw
    finally:
        os.close(fd)


def _strict_bundle(bundle: Path) -> None:
    if not bundle.is_absolute() or bundle.is_symlink() or not bundle.is_dir():
        raise OffsiteVerificationError("absolute regular bundle directory required")
    root_members = {"source.json", "manifest.json", "envelope"}
    envelope_members = {"manifest.json", "snapshot.aesgcm"}
    try:
        with os.scandir(bundle) as entries:
            root = {entry.name: entry for entry in entries}
        if set(root) != root_members or not root["envelope"].is_dir(follow_symlinks=False):
            raise OffsiteVerificationError("bundle has missing or extra entries")
        if any(not root[name].is_file(follow_symlinks=False)
               for name in ("source.json", "manifest.json")):
            raise OffsiteVerificationError("unsafe bundle member")
        with os.scandir(bundle / "envelope") as entries:
            envelope = {entry.name: entry for entry in entries}
        if set(envelope) != envelope_members or any(
                not entry.is_file(follow_symlinks=False) for entry in envelope.values()):
            raise OffsiteVerificationError("encrypted bundle has missing or extra entries")
    except OSError as exc:
        raise OffsiteVerificationError("encrypted bundle cannot be inspected") from exc


def _valid_prefix(value: object) -> bool:
    return (isinstance(value, str) and 0 < len(value) <= 512
            and all(PREFIX_SEGMENT.fullmatch(segment) is not None
                    and segment not in (".", "..") for segment in value.split("/")))


def _valid_version(value: object) -> bool:
    return (isinstance(value, str) and 1 <= len(value) <= 1024
            and value.isascii() and value[0] != "-" and value.lower() != "null"
            and all(33 <= ord(char) <= 126 for char in value))


def _load_inventory(path: Path, *, bucket: str, owner: str,
                    bundle: Path) -> list[dict]:
    if not path.is_absolute():
        raise OffsiteVerificationError("absolute inventory path required")
    _strict_bundle(bundle)
    inventory = _json(_read_small_regular(path, 32768))
    if (set(inventory) != INVENTORY_FIELDS
            or type(inventory["schema_version"]) is not int
            or inventory["schema_version"] != 1
            or inventory["bucket"] != bucket
            or inventory["expected_bucket_owner"] != owner
            or not _valid_prefix(inventory["key_prefix"])
            or not isinstance(inventory["objects"], list)
            or len(inventory["objects"]) != len(MEMBER_LIMITS)):
        raise OffsiteVerificationError("invalid or unpinned S3 inventory")
    seen_names: set[str] = set()
    seen_versions: set[str] = set()
    for entry in inventory["objects"]:
        if not isinstance(entry, dict) or set(entry) != OBJECT_FIELDS:
            raise OffsiteVerificationError("invalid S3 object inventory entry")
        name = entry["name"]
        if not isinstance(name, str) or name not in MEMBER_LIMITS or name in seen_names:
            raise OffsiteVerificationError("duplicate or unexpected bundle member")
        seen_names.add(name)
        if (entry["key"] != inventory["key_prefix"] + "/" + name
                or not _valid_version(entry["version_id"])
                or entry["version_id"] in seen_versions
                or not isinstance(entry["sha256"], str)
                or SHA256.fullmatch(entry["sha256"]) is None
                or type(entry["size_bytes"]) is not int
                or not 0 < entry["size_bytes"] <= MEMBER_LIMITS[name]):
            raise OffsiteVerificationError("invalid object key, version, digest or size")
        seen_versions.add(entry["version_id"])
        local_digest, local_size = _digest_regular(bundle / name, MEMBER_LIMITS[name])
        if local_digest != entry["sha256"] or local_size != entry["size_bytes"]:
            raise OffsiteVerificationError("inventory does not pin local bundle bytes")
    if seen_names != set(MEMBER_LIMITS):
        raise OffsiteVerificationError("inventory lacks an encrypted bundle member")
    return inventory["objects"]


def _utc(value: object) -> datetime:
    if not isinstance(value, str):
        raise OffsiteVerificationError("UTC retention timestamp required")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise OffsiteVerificationError("invalid retention timestamp") from exc
    if parsed.tzinfo is None or parsed.utcoffset().total_seconds() != 0:
        raise OffsiteVerificationError("UTC retention timestamp required")
    return parsed.astimezone(timezone.utc)


def _aws_json(aws_cli: str, region: str, operation: str,
              options: list[str], *, destination: Path | None = None) -> dict:
    command = [aws_cli, "--region", region, "--no-cli-pager", "--output", "json",
               "s3api", operation, *options]
    if destination is not None:
        command.append(str(destination))
    environment = os.environ.copy()
    environment["AWS_PAGER"] = ""
    environment["AWS_CLI_AUTO_PROMPT"] = "off"
    environment["AWS_IGNORE_CONFIGURED_ENDPOINT_URLS"] = "true"
    try:
        result = subprocess.run(command, stdin=subprocess.DEVNULL,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                text=True, timeout=120, check=False, env=environment)
    except (OSError, subprocess.TimeoutExpired, UnicodeError) as exc:
        raise OffsiteVerificationError(f"AWS {operation} unavailable") from exc
    if result.returncode != 0 or len(result.stdout) > 128 * 1024:
        raise OffsiteVerificationError(f"AWS {operation} failed")
    return _json(result.stdout)


def verify_offsite_copy(inventory: Path, bundle: Path, *, bucket: str,
                        expected_bucket_owner: str, region: str,
                        minimum_retain_until: str, aws_cli: str = "aws") -> dict:
    """Verify local pins and exact remote versions, returning bounded evidence."""
    if (not isinstance(bucket, str) or BUCKET.fullmatch(bucket) is None
            or ".." in bucket or ".-" in bucket or "-." in bucket
            or not isinstance(expected_bucket_owner, str)
            or OWNER.fullmatch(expected_bucket_owner) is None
            or not isinstance(region, str) or REGION.fullmatch(region) is None
            or not isinstance(aws_cli, str) or not aws_cli):
        raise OffsiteVerificationError("invalid independently pinned S3 target")
    minimum = _utc(minimum_retain_until)
    if minimum <= datetime.now(timezone.utc):
        raise OffsiteVerificationError("minimum retention must be in the future")
    objects = _load_inventory(inventory, bucket=bucket,
                              owner=expected_bucket_owner, bundle=bundle)
    common = ["--bucket", bucket, "--expected-bucket-owner", expected_bucket_owner]
    versioning = _aws_json(aws_cli, region, "get-bucket-versioning", common)
    if versioning.get("Status") != "Enabled":
        raise OffsiteVerificationError("S3 bucket versioning is not enabled")
    lock = _aws_json(aws_cli, region, "get-object-lock-configuration", common)
    if (not isinstance(lock.get("ObjectLockConfiguration"), dict)
            or lock["ObjectLockConfiguration"].get("ObjectLockEnabled") != "Enabled"):
        raise OffsiteVerificationError("S3 Object Lock is not enabled")
    verified = []
    with tempfile.TemporaryDirectory(prefix="ocaf-stage041-s3-") as download_dir:
        for index, entry in enumerate(objects):
            options = [*common, "--key", entry["key"],
                       "--version-id", entry["version_id"]]
            head = _aws_json(aws_cli, region, "head-object", options)
            if (head.get("VersionId") != entry["version_id"]
                    or type(head.get("ContentLength")) is not int
                    or head["ContentLength"] != entry["size_bytes"]
                    or head.get("ObjectLockMode") != "COMPLIANCE"):
                raise OffsiteVerificationError("S3 object version, size or lock mode mismatch")
            retention = _aws_json(aws_cli, region, "get-object-retention", options)
            record = retention.get("Retention")
            if not isinstance(record, dict) or record.get("Mode") != "COMPLIANCE":
                raise OffsiteVerificationError("S3 object version lacks COMPLIANCE retention")
            until = _utc(record.get("RetainUntilDate"))
            if (until < minimum or until <= datetime.now(timezone.utc)
                    or _utc(head.get("ObjectLockRetainUntilDate")) != until):
                raise OffsiteVerificationError("S3 object retention is too short or inconsistent")
            destination = Path(download_dir) / str(index)
            downloaded = _aws_json(aws_cli, region, "get-object", options,
                                   destination=destination)
            if (downloaded.get("VersionId") != entry["version_id"]
                    or type(downloaded.get("ContentLength")) is not int
                    or downloaded["ContentLength"] != entry["size_bytes"]):
                raise OffsiteVerificationError("downloaded S3 object version or size mismatch")
            remote_digest, remote_size = _digest_regular(destination, MEMBER_LIMITS[entry["name"]])
            if remote_digest != entry["sha256"] or remote_size != entry["size_bytes"]:
                raise OffsiteVerificationError("downloaded S3 bytes differ from pinned local SHA256")
            local_digest, local_size = _digest_regular(bundle / entry["name"],
                                                       MEMBER_LIMITS[entry["name"]])
            if local_digest != remote_digest or local_size != remote_size:
                raise OffsiteVerificationError("local bundle changed during S3 verification")
            verified.append({"name": entry["name"], "key": entry["key"],
                             "version_id": entry["version_id"],
                             "sha256": remote_digest, "size_bytes": remote_size,
                             "retain_until": until.isoformat().replace("+00:00", "Z")})
    _strict_bundle(bundle)
    return {"schema_version": 1, "status": "S3_OBJECT_LOCK_COPY_VERIFIED",
            "bucket": bucket, "expected_bucket_owner": expected_bucket_owner,
            "region": region, "objects": verified,
            "independent_kms_proved": False, "durable_host_restore_proved": False,
            "stage_041_complete": False, "production_approved": False}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--bucket", required=True)
    parser.add_argument("--expected-bucket-owner", required=True)
    parser.add_argument("--region", required=True)
    parser.add_argument("--minimum-retain-until", required=True,
                        help="UTC ISO-8601 timestamp from the operator retention policy")
    args = parser.parse_args()
    inventory = args.inventory if args.inventory.is_absolute() else Path.cwd() / args.inventory
    bundle = args.bundle if args.bundle.is_absolute() else Path.cwd() / args.bundle
    try:
        report = verify_offsite_copy(inventory, bundle, bucket=args.bucket,
                                     expected_bucket_owner=args.expected_bucket_owner,
                                     region=args.region,
                                     minimum_retain_until=args.minimum_retain_until)
    except OffsiteVerificationError as exc:
        parser.exit(1, f"S3 offsite copy verification denied: {exc}\n")
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
