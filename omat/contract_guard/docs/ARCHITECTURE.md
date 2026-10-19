# OMAt Contract Guard — architectural acceptance

This Rust crate contains a standalone read-only command-line policy checker. It parses the canonical registry, rejects path traversal and duplicate identities, scans ownership directories for unregistered Cargo components, and evaluates real Rust source, tests, documentation and Zenoh applicability. It does not act as a networked agent or substitute for OMAs. It never certifies deployment on the basis of file presence alone.
