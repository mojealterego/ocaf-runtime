# Stage 0.41.6 — S3 Object Lock offsite copy verifier interface

`scripts/stage041_s3_object_lock_verify.py` checks a copy of the existing
encrypted cross-kernel lab bundle. It accepts exactly four local files and
exactly four pinned S3 object versions:

| Bundle member | S3 key suffix |
| --- | --- |
| `source.json` | `source.json` |
| `manifest.json` | `manifest.json` |
| `envelope/manifest.json` | `envelope/manifest.json` |
| `envelope/snapshot.aesgcm` | `envelope/snapshot.aesgcm` |

Run the existing `stage041_encrypted_artifact_gate.py` with the ephemeral
encryption key before copying the bundle. The S3 verifier does not decrypt the
snapshot. It rejects extra files, including plaintext `snapshot.pgcustom`.

The operator supplies an inventory JSON file with exactly these top-level
fields: `schema_version` (integer `1`), `bucket`, `expected_bucket_owner`,
`key_prefix`, and `objects`. `objects` has exactly four entries, one per
member. Each entry has exactly `name`, `key`, `version_id`, `sha256`, and
`size_bytes`. A key must equal `key_prefix + "/" + name`. Use the `VersionId`
returned by each successful upload, never an unversioned or latest-object
reference. `sha256` is lowercase SHA-256 of the local file bytes and
`size_bytes` is the local byte length. Keep the inventory with the original
bundle and review the version IDs independently of the upload client.

Example shape (replace every example value with the real local hash, size,
key and upload response version ID):

```json
{
  "schema_version": 1,
  "bucket": "ocaf-stage041-offsite",
  "expected_bucket_owner": "123456789012",
  "key_prefix": "stage041/run-123",
  "objects": [
    {"name": "source.json", "key": "stage041/run-123/source.json", "version_id": "version-1", "sha256": "<64 lowercase hex characters>", "size_bytes": 123},
    {"name": "manifest.json", "key": "stage041/run-123/manifest.json", "version_id": "version-2", "sha256": "<64 lowercase hex characters>", "size_bytes": 123},
    {"name": "envelope/manifest.json", "key": "stage041/run-123/envelope/manifest.json", "version_id": "version-3", "sha256": "<64 lowercase hex characters>", "size_bytes": 123},
    {"name": "envelope/snapshot.aesgcm", "key": "stage041/run-123/envelope/snapshot.aesgcm", "version_id": "version-4", "sha256": "<64 lowercase hex characters>", "size_bytes": 123}
  ]
}
```

After the copy, run from an operator environment with AWS CLI credentials
that can read bucket versioning, Object Lock configuration, object headers,
object retention and exact object versions. Supply the bucket and 12-digit
expected owner independently of the inventory. Set the minimum retention
timestamp from the retention policy, in UTC and in the future:

```bash
python3 scripts/stage041_s3_object_lock_verify.py \
  --inventory /secure/stage041/inventory.json \
  --bundle /secure/stage041/stage041-cross-source \
  --bucket ocaf-stage041-offsite \
  --expected-bucket-owner 123456789012 \
  --region us-east-1 \
  --minimum-retain-until 2027-01-01T00:00:00Z
```

The verifier requires bucket versioning `Enabled`, Object Lock `Enabled`, and
`COMPLIANCE` retention through at least the policy timestamp on each exact
version. It checks each `head-object` and `get-object-retention` response,
downloads each pinned version into a temporary directory, and compares its
SHA-256 and size with the local bytes and inventory. Every AWS request passes
`--expected-bucket-owner`. AWS CLI calls use argument arrays without a shell;
the tool discards partial downloads when a check fails. Its successful JSON
report is copy verification only and always includes
`independent_kms_proved=false`, `durable_host_restore_proved=false`,
`stage_041_complete=false`, and `production_approved=false`.

The workflow `stage041-s3-object-lock-verifier.yml` runs offline negative tests
against a fake AWS CLI. It does not use cloud credentials or prove a live S3
copy. There is no live S3/KMS evidence in this change. Independent durable
keys, a retained offsite copy under a real account and a restore from that
copy on a durable independent host remain open for Stage 0.41.6.
