# PDF21 — 26-stronicowy audyt semantyczny OmniForge

Źródło: Architektura OmniContext Agent Framework (OCAF) w Języku Rust: Paradygmaty, Skalowalność i Bezpieczeństwo (plik użytkownika „Architektura OmniContext Agent Framework (OCAF) w ..(1) (1).PDF”). Data 2026-10-16.

| Strony | Treść / obowiązek | Etapy | Status i dowód wymagany |
|---|---|---|---|
| 1–4 | Rust/Tokio/Zenoh SHM, TypedLayout, CRDT Automerge/Yrs, DAG/Bloom/delta | 0.46–0.47 | E2E real SHM, version compatibility, tenant isolation, partitions/recovery |
| 4–5 | OTLP, Tokio spawn instrumentation | 0.42 | exporter, propagated spans across RPC |
| 5–6 | MCP dynamic routing, Tree-sitter CST/type-sitter | 0.43, 0.67 | semantic JSON Schema, authZ, hostile metadata |
| 6–8 | OAuth PKCE, secret separation, ZKP | 0.44–0.45 | actual circuit/verifier and negative tenant tests |
| 8–9 | Firecracker/KVM, Aya/eBPF | 0.39, 0.55 | real guest, broker, policy, host-vs-guest visibility proof |
| 9–10 | L402 Lightning Macaroons | 0.49 | mocked_*, fixed preimage are not settlement |
| 10–11 | RAG, OWASP ASI | 0.63–0.64, 0.67 | constant embedding vec![0.5;1024] invalid |
| 11–12 | ML-KEM/ML-DSA, Loihi 2 SNN | 0.45, 0.91 | negotiated crypto/HIL; toy SNN not Loihi |
| 12–15 | Six-phase roadmap, Cargo.toml and OcafAgent code sketch | 0.39, 0.43–0.49, 0.98 | do not replace existing runtime; latency numbers unverified |
| 15–16 | OmniDev: Architect, Critic, Executor | 0.43, 0.55, 0.63, 0.67, 0.98 | AST_MAP/git diff/true are stubs |
| 17–18 | OmniUX: Figma MCP, CST frontend, visual critic | 0.43, 0.50–0.55, 0.98 | FIGMA_AST/<Btn/>/0.99 are stubs |
| 18–20 | OmniE2E: SwiftUI, Kotlin Compose, QA self-healing | 0.50–0.55, 0.98 | xcodebuild_source/gradlew_source and self-healing prints are stubs; Xcode needs macOS |
| **20–23** | **NOWE: OmniForge**: InzynierSilnika_CPP_Rust (Bevy Rust ECS / Unreal C++), ArchitektShaderow_GPU (GLSL/HLSL, headless Vulkan), RezyserZasobow_3D (PBR/mesh purchases via L402), deterministic physics ECS ticks | **0.43, 0.46–0.47, 0.49–0.50, 0.55, 0.63, 0.67, 0.98** | real version-pinned Bevy/Unreal builds, shader compiler + GPU, physical determinism, authorized settlement/license/provenance, bounded sandbox |
| 23–24 | Four-swarm architecture OmniDev → OmniUX → OmniE2E + OmniForge | 0.50, 0.55, 0.98 | typed handoffs, role limits, audit and acceptance |
| 24–26 | Bibliography (55 citations) | — | reference list, not integrated adapters |

## Code-vs-claim check

OmniForge example returns constant `AST_SILNIKA_100%_UPDATED`, `system_ruch_gracza.rs`, `compute_shader.glsl`, `/assets/model_wygenerowany.glb`, and `framerate: 144.0`. It performs no actual repo analysis, compilation, shader benchmarking, asset purchase, physics simulation or release. Source claims about 60 FPS, AAA and outperforming competitors are unverified. Headless Vulkan cannot be presumed available inside Firecracker guest without explicit GPU capability. A purchase requires human-approved policy, scoped tenant budget, verified invoice/preimage, licensing, audit and refund. A parser does not guarantee compatibility with changing engine APIs.

## Mapping and acceptance

Canonical stage definitions in `docs/roadmap/canonical_v11_inventory.json` contain explicit PDF21 OmniForge scope and acceptance for nine existing stages; separate gaps recorded in `missing_capabilities_v11.json`. OmniForge is a **fourth specialized swarm**, not an additional Agents Builder product. Ten Cloud/Windows/macOS/Android/iOS Code/NoCode products remain separate. All new work PLANNED_OR_UNVERIFIED until code, tests, CI SHA and target-environment evidence exist.
