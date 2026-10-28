# Asterion installer contract

`build.sh` requires a Rust toolchain and the OCAF repository layout. It runs
`cargo test --locked` and a release build, packages the binary, configuration,
documentation, and certificate record, and emits both internal `SHA256SUMS`
and an archive `.sha256` sidecar.

`install.sh <archive>` installs into `OCAF_AGENT_PREFIX` or `$HOME/.local`.
The prefix must be an absolute canonical path other than `/`, with no
symlinked destination directories or files. The same path guard applies to
uninstall.
It accepts only the fixed archive paths and regular files/directories, requires
checksum entries for the binary and every bundled file, verifies those checksums,
and runs the packaged `status` command before replacing
an existing local installation. A failed replacement restores the previous
binary and share directory. `uninstall.sh` removes only the installed binary
and share directory under that prefix.

No archive signature or trusted publisher key is supplied. The checksum is an
integrity check, not authenticity evidence. Installation keeps the agent in
`DRAFT` and does not touch OMAs, OMAt, OSTc, or source repositories.
