# Architecture — OCAF Component Gate

The program is a standalone, dependency-free Rust CLI. It discovers component directories under `components/{agents,systems,swarms,omat,omas}/`, validates each artifact's manifest, Rust package and executable source, Rust integration tests, the three required documentation categories, and the Zenoh integration contract for distributed swarms and OMAs servers.

A checked-in target registry in `ops/OCAF_REQUIRED_COMPONENTS.tsv` records the 33 Make engineering groups as *required*. A missing implementation is always reported PENDING; in formal strict mode it is a hard failure. An existing folder containing only Markdown is a hard failure even in migration mode.

`transport = "zenoh"` requires both a Cargo Zenoh dependency and source integration. The structural checker does not replace cargo compilation, negative testing, live Zenoh exchange, secure identity binding, CI log review, independent certification or operator acceptance.

The gate itself is an OMAt tool with its own real Rust implementation and tests. It is offline and therefore correctly declares `transport = "none"`.
