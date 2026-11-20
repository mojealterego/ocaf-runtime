//! Native candidate package for aml-compliance.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-aml-compliance"#, source_id: r#"aml-compliance"#, name: r#"AML Compliance Review Agent"#,
    specialization: r#"Transaction-alert triage, evidence mapping and compliance escalation drafting."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/occupational-ai/agent.py"#,
    instructions: r#"Finance
Triage transaction alerts, compare behavior with documented customer/business context, map evidence and prepare an escalation or SAR draft for authorized compliance staff.
Do not autonomously freeze, block or move funds; escalation and reporting decisions require authorized review.
Do not use protected traits as stand-alone risk signals.
Keep sensitive financial data within approved systems; this module has no external data connector."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
