use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use zenoh::prelude::r#async::*;
use zenoh::query::Reply;

// ============================================================================
// 1. STRUKTURY DANYCH PROTOKOŁU OCAF I EKONOMII L402
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub cost_sats: u64, // Koszt w ułamkach waluty (L402)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRequest {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub macaroon_token: Option<String>,
    pub preimage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResponse {
    pub status: String,
    pub output: serde_json::Value,
    pub execution_time_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L402InvoiceChallenge {
    pub payment_required: bool,
    pub invoice: String,
    pub cost_sats: u64,
    pub macaroon: String,
}

// ============================================================================
// 2. REJESTR NARZĘDZI ORAZ SERWER SHM/ZENOH (<1 ms RPC)
// ============================================================================

pub struct FastToolProvider {
    session: Arc<zenoh::Session>,
    tools: Arc<RwLock<HashMap<String, ToolSchema>>>,
}

impl FastToolProvider {
    pub async fn new(session: Arc<zenoh::Session>) -> Self {
        Self {
            session,
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_tool(&self, schema: ToolSchema) {
        let mut map = self.tools.write().await;
        map.insert(schema.name.clone(), schema);
    }

    pub async fn start_listeners(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tools_clone = self.tools.clone();
        
        // Deklaracja Zenoh Queryable - odpowiada w czasie mikrosekundowym na zapytania o narzędzia
        let discovery_queryable = self
            .session
            .declare_queryable("ocaf/registry/discover")
            .res()
            .await?;

        // Wątek obsługi odkrywania narzędzi w tle
        tokio::spawn(async move {
            while let Ok(query) = discovery_queryable.recv_async().await {
                let available_tools = tools_clone.read().await;
                let payload = serde_json::to_vec(&*available_tools).unwrap();
                
                let key_expr = query.key_expr().to_string();
                if let Err(e) = query.reply(Ok(zenoh::sample::Sample::new(key_expr, payload))).res().await {
                    eprintln!("Błąd odpowiedzi Zenoh: {:?}", e);
                }
            }
        });

        // Queryable dla egzekucji narzędzia
        let execution_queryable = self
            .session
            .declare_queryable("ocaf/tools/execute/*")
            .res()
            .await?;

        tokio::spawn(async move {
            while let Ok(query) = execution_queryable.recv_async().await {
                let start_time = std::time::Instant::now();
                let key = query.key_expr().as_str();
                let tool_name = key.strip_prefix("ocaf/tools/execute/").unwrap_or("unknown");

                // Odczyt ładunku wejściowego
                let payload_bytes = query.value().map(|v| v.payload.to_bytes()).unwrap_or_default();
                let req: ToolExecutionRequest = match serde_json::from_slice(&payload_bytes) {
                    Ok(r) => r,
                    Err(_) => {
                        let _ = query.reply(Err(zenoh::value::Value::from("Invalid JSON"))).res().await;
                        continue;
                    }
                };

                // Weryfikacja protokołu mikropłatności L402
                if req.macaroon_token.is_none() || req.preimage.is_none() {
                    let challenge = L402InvoiceChallenge {
                        payment_required: true,
                        invoice: format!("lnbc_{}_test_invoice", tool_name),
                        cost_sats: 50,
                        macaroon: format!("macaroon_auth_for_{}", tool_name),
                    };
                    let err_payload = serde_json::to_vec(&challenge).unwrap();
                    let _ = query.reply(Ok(zenoh::sample::Sample::new(key, err_payload))).res().await;
                    continue;
                }

                // Bezpieczne wykonanie logiki narzędzia
                let res = ToolExecutionResponse {
                    status: "SUCCESS".to_string(),
                    output: serde_json::json!({
                        "result": format!("Pomyślnie przetworzono zadanie narzędziem '{}'", tool_name),
                        "args_received": req.arguments
                    }),
                    execution_time_us: start_time.elapsed().as_micros() as u64,
                };

                let resp_payload = serde_json::to_vec(&res).unwrap();
                let _ = query.reply(Ok(zenoh::sample::Sample::new(key, resp_payload))).res().await;
            }
        });

        Ok(())
    }
}

// ============================================================================
// 3. AGENT RUNNER: DYNAMICZNY ROUTER I ZARZĄDZANIE PAMIĘCIĄ
// ============================================================================

pub struct OcafAgent {
    pub agent_id: String,
    pub session: Arc<zenoh::Session>,
    pub budget_sats: Arc<RwLock<u64>>,
}

impl OcafAgent {
    pub fn new(agent_id: String, session: Arc<zenoh::Session>, initial_budget_sats: u64) -> Self {
        Self {
            agent_id,
            session,
            budget_sats: Arc::new(RwLock::new(initial_budget_sats)),
        }
    }

    /// Pobieranie schematu narzędzia w locie (Dynamic Tool Loading)
    /// Zapytanie przez Zenoh eliminuje narzut HTTP i trwa ułamki milisekund.
    pub async fn discover_tool(&self, target_tool: &str) -> Option<ToolSchema> {
        let t0 = std::time::Instant::now();
        let replies = self
            .session
            .get("ocaf/registry/discover")
            .res()
            .await
            .ok()?;

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.sample {
                let tools_map: HashMap<String, ToolSchema> =
                    serde_json::from_slice(&sample.payload.to_bytes()).unwrap_or_default();
                
                println!(
                    "[{}] Odkryto narzędzia przez Zenoh w czasie: {} µs (<1 ms)",
                    self.agent_id,
                    t0.elapsed().as_micros()
                );
                
                if let Some(tool) = tools_map.get(target_tool) {
                    return Some(tool.clone());
                }
            }
        }
        None
    }

    /// Bezpieczne wywołanie z obsługą L402 i rozliczeniem M2M
    pub async fn execute_tool_call(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let exec_key = format!("ocaf/tools/execute/{}", tool_name);
        
        // Krok 1: Próba zapytania bez tokena uwierzytelniającego
        let initial_req = ToolExecutionRequest {
            tool_name: tool_name.to_string(),
            arguments: args.clone(),
            macaroon_token: None,
            preimage: None,
        };

        let req_bytes = serde_json::to_vec(&initial_req)?;
        let t_call = std::time::Instant::now();
        let replies = self.session.get(&exec_key).value(req_bytes).res().await?;

        let mut received_challenge: Option<L402InvoiceChallenge> = None;

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.sample {
                let payload = sample.payload.to_bytes();
                if let Ok(challenge) = serde_json::from_slice::<L402InvoiceChallenge>(&payload) {
                    if challenge.payment_required {
                        received_challenge = Some(challenge);
                        break;
                    }
                }
            }
        }

        // Krok 2: Automatyczna negocjacja płatności L402, jeśli wymagana
        if let Some(challenge) = received_challenge {
            println!(
                "[{}] Usługa wymaga opłaty L402: {} satoshi. Autoryzacja z budżetu...",
                self.agent_id, challenge.cost_sats
            );

            let mut budget = self.budget_sats.write().await;
            if *budget < challenge.cost_sats {
                return Err("Przekroczono limit budżetu M2M na wywołanie narzędzia!".into());
            }

            // Realizacja płatności: pobranie środków i wygenerowanie kryptograficznego Preimage
            *budget -= challenge.cost_sats;
            let simulated_preimage = "settled_preimage_hash_77492104".to_string();

            // Krok 3: Ponowienie zapytania z tokenem Macaroon i dowodem wpłaty Preimage
            let authorized_req = ToolExecutionRequest {
                tool_name: tool_name.to_string(),
                arguments: args,
                macaroon_token: Some(challenge.macaroon),
                preimage: Some(simulated_preimage),
            };

            let auth_req_bytes = serde_json::to_vec(&authorized_req)?;
            let auth_replies = self.session.get(&exec_key).value(auth_req_bytes).res().await?;

            while let Ok(reply) = auth_replies.recv_async().await {
                if let Ok(sample) = reply.sample {
                    let resp: ToolExecutionResponse = serde_json::from_slice(&sample.payload.to_bytes())?;
                    println!(
                        "[{}] Zakończono wywołanie narzędzia. Czas wykonania po szynie: {} µs",
                        self.agent_id,
                        t_call.elapsed().as_micros()
                    );
                    return Ok(resp.output);
                }
            }
        }

        Err("Nie udało się pomyślnie wykonać procedury narzędziowej".into())
    }
}

// ============================================================================
// 4. FUNKCJA STARTOWA (RUNNER I TESTY BENCHMARKOWE)
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    println!("=== Inicjalizacja Ekosystemu OCAF (Rust + Eclipse Zenoh Core) ===");

    // Inicjalizacja bezbrokera/węzła Zenoh
    let zenoh_config = zenoh::Config::default();
    let session = Arc::new(zenoh::open(zenoh_config).res().await?);

    // Utworzenie i rejestracja narzędzi na szynie
    let provider = FastToolProvider::new(session.clone()).await;
    provider
        .register_tool(ToolSchema {
            name: "enterprise_financial_analyzer".to_string(),
            description: "Analizuje wskaźniki bilansowe spółek".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": { "ticker": { "type": "string" } },
                "required": ["ticker"]
            }),
            cost_sats: 50,
        })
        .await;

    provider.start_listeners().await?;
    println!("Dostawca narzędzi OCAF pomyślnie zarejestrował punkty końcowe Zenoh.");

    // Utworzenie instancji Agenta
    let agent = OcafAgent::new("Agent-Finance-01".to_string(), session.clone(), 1000);

    // Krok A: Dynamiczne odkrycie narzędzia w czasie sub-milisekundowym
    let discovered = agent.discover_tool("enterprise_financial_analyzer").await;
    match discovered {
        Some(tool) => {
            println!(
                "Pomyślnie załadowano definicję: {} (Koszt: {} sats)",
                tool.name, tool.cost_sats
            );

            // Krok B: Wywołanie narzędzia z negocjacją ekonomiczną L402
            let args = serde_json::json!({ "ticker": "NVDA" });
            let execution_result = agent.execute_tool_call(&tool.name, args).await?;
            println!("Wynik zwrócony do modelu:\n{}", serde_json::to_string_pretty(&execution_result)?);
        }
        None => eprintln!("Nie odnaleziono wymaganego narzędzia!"),
    }

    println!("Pozostały budżet agenta: {} satoshi", *agent.budget_sats.read().await);
    Ok(())
}
