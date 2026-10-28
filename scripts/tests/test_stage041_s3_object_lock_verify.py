"""Adversarial offline tests for the Stage 0.41 S3 Object Lock verifier."""
import base64
from contextlib import contextmanager
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage041_s3_object_lock_verify import (MEMBER_LIMITS, OffsiteVerificationError,
                                            verify_offsite_copy)


FAKE_AWS = r'''#!/usr/bin/env python3
import base64
import json
import os
from pathlib import Path
import sys

argv = sys.argv[1:]
with open(os.environ["FAKE_AWS_LOG"], "a", encoding="utf-8") as log:
    log.write(json.dumps(argv) + "\n")
state = json.loads(Path(os.environ["FAKE_AWS_STATE"]).read_text())
if not all(flag in argv for flag in ("--region", "--no-cli-pager", "--output", "s3api")):
    sys.exit(2)
operation = argv[argv.index("s3api") + 1]
if state.get("fail_operation") == operation:
    sys.exit(7)
def option(name):
    index = argv.index(name)
    return argv[index + 1]
if option("--expected-bucket-owner") != state["owner"] or option("--bucket") != state["bucket"]:
    sys.exit(4)
if operation == "get-bucket-versioning":
    result = {"Status": state["versioning"]}
elif operation == "get-object-lock-configuration":
    result = {"ObjectLockConfiguration": {"ObjectLockEnabled": state["lock_enabled"]}}
else:
    key = option("--key")
    obj = state["objects"].get(key)
    if obj is None or option("--version-id") != obj["version_id"]:
        sys.exit(5)
    data = base64.b64decode(obj["bytes"])
    if operation == "head-object":
        result = {"VersionId": obj.get("head_version", obj["version_id"]),
                  "ContentLength": obj.get("head_size", len(data)),
                  "ObjectLockMode": obj.get("head_mode", obj["mode"]),
                  "ObjectLockRetainUntilDate": obj.get("head_until", obj["until"])}
    elif operation == "get-object-retention":
        result = {"Retention": {"Mode": obj["mode"], "RetainUntilDate": obj["until"]}}
    elif operation == "get-object":
        Path(argv[-1]).write_bytes(data)
        result = {"VersionId": obj.get("download_version", obj["version_id"]),
                  "ContentLength": obj.get("download_size", len(data))}
    else:
        sys.exit(6)
print(json.dumps(result))
'''


class S3ObjectLockTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.bundle = root / "bundle"
        (self.bundle / "envelope").mkdir(parents=True)
        self.inventory_path = root / "inventory.json"
        self.fake = root / "aws"
        self.fake.write_text(FAKE_AWS)
        self.fake.chmod(0o700)
        self.state_path = root / "fake-state.json"
        self.log = root / "fake-log.jsonl"
        self.bucket = "ocaf-stage041-offsite"
        self.owner = "123456789012"
        self.prefix = "stage041/run-123"
        self.inventory = {"schema_version": 1, "bucket": self.bucket,
                          "expected_bucket_owner": self.owner,
                          "key_prefix": self.prefix, "objects": []}
        self.state = {"owner": self.owner, "bucket": self.bucket,
                      "versioning": "Enabled", "lock_enabled": "Enabled", "objects": {}}
        for index, name in enumerate(MEMBER_LIMITS):
            data = (name + "\n").encode() if name != "envelope/snapshot.aesgcm" else b"encrypted bytes\x00\x01"
            (self.bundle / name).write_bytes(data)
            key = self.prefix + "/" + name
            version = f"version-{index + 1}"
            self.inventory["objects"].append({
                "name": name, "key": key, "version_id": version,
                "sha256": hashlib.sha256(data).hexdigest(), "size_bytes": len(data)})
            self.state["objects"][key] = {
                "version_id": version, "bytes": base64.b64encode(data).decode(),
                "mode": "COMPLIANCE", "until": "2100-01-01T00:00:00Z"}

    def _write(self):
        self.inventory_path.write_text(json.dumps(self.inventory) + "\n")
        self.state_path.write_text(json.dumps(self.state) + "\n")

    @contextmanager
    def _fake_environment(self):
        with patch.dict(os.environ, {"FAKE_AWS_STATE": str(self.state_path),
                                    "FAKE_AWS_LOG": str(self.log)}):
            yield

    def _verify(self):
        self._write()
        with self._fake_environment():
            return verify_offsite_copy(
                self.inventory_path, self.bundle, bucket=self.bucket,
                expected_bucket_owner=self.owner, region="us-east-1",
                minimum_retain_until="2099-01-01T00:00:00Z", aws_cli=str(self.fake))

    def _first(self):
        return self.state["objects"][self.inventory["objects"][0]["key"]]

    def test_exact_four_versions_download_and_verify(self):
        report = self._verify()
        self.assertEqual(report["status"], "S3_OBJECT_LOCK_COPY_VERIFIED")
        self.assertEqual(len(report["objects"]), 4)
        self.assertFalse(report["stage_041_complete"])
        self.assertFalse(report["independent_kms_proved"])
        calls = [json.loads(line) for line in self.log.read_text().splitlines()]
        self.assertEqual(len(calls), 14)
        self.assertEqual(sum("get-object" in call for call in calls), 4)
        for call in calls:
            self.assertEqual(call[call.index("--expected-bucket-owner") + 1], self.owner)
            self.assertEqual(call[call.index("--bucket") + 1], self.bucket)

    def test_inventory_rejects_missing_extra_duplicate_and_unpinned_members(self):
        original = deepcopy(self.inventory)
        mutations = []
        missing = deepcopy(original); missing["objects"].pop(); mutations.append(missing)
        duplicate = deepcopy(original); duplicate["objects"][1] = deepcopy(duplicate["objects"][0]); mutations.append(duplicate)
        extra = deepcopy(original); extra["objects"][0]["surprise"] = True; mutations.append(extra)
        bad_key = deepcopy(original); bad_key["objects"][0]["key"] = "other/source.json"; mutations.append(bad_key)
        bad_sha = deepcopy(original); bad_sha["objects"][0]["sha256"] = "0" * 64; mutations.append(bad_sha)
        null_version = deepcopy(original); null_version["objects"][0]["version_id"] = "null"; mutations.append(null_version)
        duplicate_version = deepcopy(original); duplicate_version["objects"][1]["version_id"] = duplicate_version["objects"][0]["version_id"]; mutations.append(duplicate_version)
        for changed in mutations:
            with self.subTest(changed=changed):
                self.inventory = changed
                with self.assertRaises(OffsiteVerificationError):
                    self._verify()
        self.assertFalse(self.log.exists(), "invalid inventory must cause zero AWS calls")

    def test_duplicate_json_field_and_extra_local_file_are_rejected(self):
        self._write()
        raw = self.inventory_path.read_text()
        self.inventory_path.write_text(raw.replace('"schema_version": 1,', '"schema_version": 1, "schema_version": 1,'))
        with self._fake_environment(), self.assertRaises(OffsiteVerificationError):
            verify_offsite_copy(self.inventory_path, self.bundle, bucket=self.bucket,
                                expected_bucket_owner=self.owner, region="us-east-1",
                                minimum_retain_until="2099-01-01T00:00:00Z",
                                aws_cli=str(self.fake))
        (self.bundle / "snapshot.pgcustom").write_bytes(b"plaintext")
        with self.assertRaises(OffsiteVerificationError):
            self._verify()
        self.assertFalse(self.log.exists())

    def test_symlinked_or_mutated_local_bytes_are_rejected(self):
        local = self.bundle / "envelope/snapshot.aesgcm"
        local.write_bytes(local.read_bytes() + b"tamper")
        with self.assertRaises(OffsiteVerificationError):
            self._verify()
        local.unlink()
        local.symlink_to(self.bundle / "source.json")
        with self.assertRaises(OffsiteVerificationError):
            self._verify()
        self.assertFalse(self.log.exists())

    def test_wrong_owner_versioning_and_lock_configuration_denied(self):
        variants = (("owner", "999999999999"), ("versioning", "Suspended"),
                    ("lock_enabled", "Disabled"))
        for field, value in variants:
            with self.subTest(field=field):
                self.state[field] = value
                with self.assertRaises(OffsiteVerificationError):
                    self._verify()
                self.state[field] = {"owner": self.owner, "versioning": "Enabled",
                                     "lock_enabled": "Enabled"}[field]

    def test_governance_expired_short_and_inconsistent_retention_denied(self):
        original = deepcopy(self._first())
        for mutation in ({"mode": "GOVERNANCE"},
                         {"until": "2020-01-01T00:00:00Z"},
                         {"until": "2090-01-01T00:00:00Z"},
                         {"head_until": "2101-01-01T00:00:00Z"},
                         {"head_mode": "GOVERNANCE"}):
            with self.subTest(mutation=mutation):
                self._first().update(mutation)
                with self.assertRaises(OffsiteVerificationError):
                    self._verify()
                self._first().clear(); self._first().update(original)

    def test_wrong_head_or_download_version_and_remote_byte_tamper_denied(self):
        original = deepcopy(self._first())
        for mutation in ({"head_version": "other-version"},
                         {"download_version": "other-version"},
                         {"bytes": base64.b64encode(b"forged bytes").decode(),
                          "head_size": original_size(self._first()),
                          "download_size": original_size(self._first())}):
            with self.subTest(mutation=mutation):
                self._first().update(mutation)
                with self.assertRaises(OffsiteVerificationError):
                    self._verify()
                self._first().clear(); self._first().update(original)

    def test_cli_failure_denies_and_opaque_version_is_passed_as_one_argument(self):
        self.state["fail_operation"] = "get-object-retention"
        with self.assertRaises(OffsiteVerificationError):
            self._verify()
        self.state.pop("fail_operation")
        sentinel = Path(self.temp.name) / "shell-would-create-this"
        weird = "v1;$(touch>" + str(sentinel) + ")"
        self.inventory["objects"][0]["version_id"] = weird
        self._first()["version_id"] = weird
        self._verify()
        self.assertFalse(sentinel.exists())
        calls = [json.loads(line) for line in self.log.read_text().splitlines()]
        self.assertTrue(any(weird in call for call in calls))

    def test_cli_emits_partial_evidence_only(self):
        self._write()
        with self._fake_environment():
            result = subprocess.run(
                [sys.executable, str(Path(__file__).resolve().parents[1] /
                                     "stage041_s3_object_lock_verify.py"),
                 "--inventory", str(self.inventory_path), "--bundle", str(self.bundle),
                 "--bucket", self.bucket, "--expected-bucket-owner", self.owner,
                 "--region", "us-east-1", "--minimum-retain-until", "2099-01-01T00:00:00Z"],
                capture_output=True, text=True, check=False,
                env={**os.environ, "PATH": self.temp.name + os.pathsep + os.environ["PATH"]})
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertFalse(report["stage_041_complete"])
        self.assertFalse(report["production_approved"])


def original_size(obj):
    return len(base64.b64decode(obj["bytes"]))


if __name__ == "__main__":
    unittest.main()
