# Intake: Drive PDF on OCAF services and stages 0.44–0.49

Source: public Drive folder `1EKb08OvnajBbhiBLiuWuhwInUy4odc19`, file
`Architektura OCAF_ OAuth i Usługi - Google Gemini.PDF.pdf`, 28 pages.
Downloaded PDF SHA-256:
`e3c74e0b5b0f68fa31f5c91ac6bb446cdf495e1b66cd724cc66953d099c1a8ee`.

The document proposes an active dispatcher, OAuth/OIDC/PKCE/MFA/Vault (0.44),
capability authorization (0.45), WASI/Firecracker isolation (0.46), Zenoh and
Merkle replication (0.47), bounded checking (0.48), and adaptive memory (0.49).
It is design input, not deployment or test evidence. The existing 0.44 code and
acceptance limits are documented in `STAGE_044_OAUTH_SECURITY.md` and
`deploy/stage044/README.md`; they remain not production accepted.

## Implementation claims requiring independent evidence

| PDF example | Observable behavior in example | Required evidence before claiming operation |
| --- | --- | --- |
| `InKernelNative` | Reverses input bytes | A specified native service with real semantics and isolation |
| `WasmSandbox` | Prepends `WASM_SANITY_PASS::` | Wasmtime execution, fuel enforcement, memory limits and tests |
| `FirecrackerMicroVm` | Spawns `sh -c echo` | Actual VMM startup, guest execution and isolation tests |
| `McpStreamableHttp` / `ExternalSecureApi` | Returns formatted status text | Authenticated network request, schema validation and receipt |
| `HashiCorpVaultClient` | Constructs a secret string and prints a store message | Real Vault read/write with scoped identity and live acceptance |
| Formal verification | Example bounded search over a small operation model | Explicit model, explored bound and counterexample/proof artifact; no general proof claim |

The PDF does not supersede the validated `CATALOG_ONLY` 001–555 registry.
An active dispatcher must use signed manifests, tenant and sender binding,
revocation, policy checks and the OSTc gate already established for OMAt.
No sample dispatcher response may set a production-ready or verified flag.
