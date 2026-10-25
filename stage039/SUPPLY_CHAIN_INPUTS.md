# Stage 0.39 input SBOM and provenance scope

`scripts/stage039_supply_chain.py` runs after the two WASI guest images and
their lab-signed manifests are prepared. It verifies the Ed25519 envelopes
against the configured lab public key, compares each signed payload to the
reviewed request, rechecks every SHA-pinned Firecracker/kernel/rootfs/Aya
loader/Wasmtime/module asset, and inventories the three Cargo lockfiles plus
the reviewed Aya, WASI negative, egress, quarantine, and guest build sources. It writes a deterministic CycloneDX 1.5
JSON file for the same input bytes and a separate JSON evidence report. Its
component names are logical names; no private local paths or key material are
exported in the SBOM.

The file reader holds an `O_NOFOLLOW` descriptor and checks device, inode,
size and modification/change timestamps both before and after hashing. It
also checks that the named path still points to the same inode. Tests mutate
the asset or signed envelope and mutate a file while it is being read; all
must fail before producing a success report.

This is a **prelaunch input inventory**. It does not prove that Firecracker
eventually consumed those exact inodes, reproduce the Rust toolchain binaries,
perform a vulnerability or license review, test guest escape, or represent an
independent security review. The signing key in the automated KVM lab is
ephemeral. The report therefore retains `running_inode_verified=false`,
`independent_signer_verified=false`, `stage_039_complete=false` and
`production_approved=false`.
