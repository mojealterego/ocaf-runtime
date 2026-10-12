# OmniContext Agent Framework (OCAF) - Rust + Zenoh Runtime

Ekosystem agentowy nowej generacji z dynamicznym ładowaniem narzędzi, ekonomią M2M (L402) i komunikacją sub-milisekundową.

## Architektura

### Faza 1: Silnik Podstawowy + Szyna Zenoh
- **Runtime**: Rust + Tokio (zero-GC, deterministyczne)
- **Transport**: Eclipse Zenoh (<1 ms IPC/LAN)
- **Queryables**: Registry discovery + tool execution
- **L402**: Wbudowana autoryzacja i rozliczanie M2M

## Struktury Kluczowe

### ToolSchema
```rust
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,  // JSON Schema 2020-12
    pub cost_sats: u64,  // L402 pricing
}
```

### OcafAgent
```rust
pub struct OcafAgent {
    pub agent_id: String,
    pub session: Arc<zenoh::Session>,
    pub budget_sats: Arc<RwLock<u64>>,
}
```

## Przepływ Wykonania

1. **Discovery**: Agent odbiera listę narzędzi przez Zenoh Queryable (<100 µs)
2. **Authorization**: Sprawdzenie L402 challenge (invoice + macaroon)
3. **Payment**: Automatyczna negocjacja budżetu M2M
4. **Execution**: Wywołanie narzędzia z preimage validation

## Budowanie

```bash
cargo build --release
cargo run
```

## Benchmarki Opóźnień

- Zenoh discovery: < 500 µs
- L402 negotiation: < 1 ms total
- Tool execution round-trip: < 5 ms (na SHM/LAN)

## Roadmapa

- **Faza 1** ✓ Silnik Zenoh + L402
- **Faza 2**: Dynamic Tool Loading + JSON Schema resolver
- **Faza 3**: OAuth 2.1 + PKCE bridge
- **Faza 4**: Firecracker sandbox integration
- **Faza 5**: Enterprise RAG connector
- **Faza 6**: OWASP ASI compliance

## Licencja

MIT
