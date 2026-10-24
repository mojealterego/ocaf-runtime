# Stage 0.39.23: operator signed guest manifest admission

The broker runner has two explicit signing modes. `lab` accepts the CI
ephemeral Ed25519 signer and continues to report `production_approved=false`.
`operator` requires two independently supplied signed manifests, a separate
operator key ID, and the SHA-256 of a trusted public key from an operator
policy. It reads all three files through `O_NOFOLLOW` descriptors, checks
owner/mode/size and non-mutable parent paths, verifies both signatures and
compares their payloads with the manifests used to reserve the broker lease.
These checks happen **before** SQLite reservation or VM launch.

Example on a privileged KVM host after both tenant images and exact manifest
payloads are already staged:

```sh
python3 scripts/stage039_broker_vm_binding.py \
  --signer-mode operator \
  --trusted-key-id operator-2026 \
  --trusted-public-key /etc/ocaf/stage039/operator-ed25519.pub \
  --trusted-public-key-sha256 SHA256_FROM_INDEPENDENT_OPERATOR_POLICY \
  --signed-manifest-a /opt/ocaf039-real-EXAMPLE/approved/signed-wasi-a.json \
  --signed-manifest-b /opt/ocaf039-real-EXAMPLE/approved/signed-wasi-b.json \
  [all existing real-KVM broker runner arguments]
```

The signer must sign `stage039_signed_manifest.canonical_payload()` for each
exact `request-wasi-a.json` and `request-wasi-b.json`; the envelope format is
`{key_id,public_key,signature,payload}` with base64 Ed25519 public key and
signature. The private key is never an input to the runner or stored in this
repository. The independent policy must pin the public key hash outside the
manifest and preserve its provenance. The host must be able to keep the
generated images and manifests stable while the operator signs; the ephemeral
GitHub-hosted lab workflow is not that handoff.

The mode proves cryptographic pre-admission checks, not that an independent
operator actually provisioned the key in a real deployment. It does not
upgrade stage 0.39, multi-host acceptance or production approval. Those
require externally supplied assets and an observed real KVM run in this mode.
