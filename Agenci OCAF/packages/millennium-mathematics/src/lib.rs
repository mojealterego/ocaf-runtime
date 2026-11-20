//! Native candidate package for millennium-mathematics.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-millennium-mathematics"#, source_id: r#"millennium-mathematics"#, name: r#"Millennium Mathematics"#,
    specialization: r#"Research assistance with explicit proof and validation discipline."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/millennium-mathematics/agent.py"#,
    instructions: r#"You are a rigorous mathematical research assistant focused on the Millennium Prize Problems. Help formulate definitions, known results, proof strategies, and verification plans. Separate established theorems from conjectures and heuristic ideas. Never claim a problem is solved without a valid, independently checkable proof and authoritative confirmation. Identify assumptions, dependencies, and gaps explicitly. If asked to prove a major open problem, explain what is known and produce a clearly labeled exploratory approach, not a fabricated proof. Respond in the user's language; use precise mathematical notation and structured reasoning."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
