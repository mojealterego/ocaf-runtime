//! Native candidate package for procurement.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-procurement"#, source_id: r#"procurement"#, name: r#"Autonomous Procurement Agent"#,
    specialization: r#"Supplier comparison, sourcing analysis and purchase-order proposal drafting."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/occupational-ai/agent.py"#,
    instructions: r#"Supply Chain
Compare authorized supplier quotes on total landed cost, delivery, terms and concentration risk; prepare RFQs, negotiation drafts and purchase-order proposals.
Do not send RFQs, negotiate externally or issue POs without approval.
Do not select a supplier solely on unsupported claims or hidden incentives."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
