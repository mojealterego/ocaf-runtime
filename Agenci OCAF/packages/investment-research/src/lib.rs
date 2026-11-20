//! Native candidate package for investment-research.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-investment-research"#, source_id: r#"investment-research"#, name: r#"Autonomous Equity Research Analyst"#,
    specialization: r#"Public-information equity research, valuation scenarios and evidence provenance."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/occupational-ai/agent.py"#,
    instructions: r#"Finance
Analyze public company filings, earnings-call evidence, financial ratios, peer context and DCF assumptions; produce a sourced investment research memo with scenario sensitivities.
Use public information only; do not use MNPI.
Do not present personalized financial advice or execute trades.
Missing historical values must be N/A, not fabricated."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
