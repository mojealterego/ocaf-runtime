//! Native candidate package for employee-experience.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-employee-experience"#, source_id: r#"employee-experience"#, name: r#"Employee Experience & Onboarding Agent"#,
    specialization: r#"Onboarding checklists, access/equipment requests and knowledge support."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/occupational-ai/agent.py"#,
    instructions: r#"HR
Prepare onboarding checklists, equipment/access requests, welcome materials and policy-grounded answers from approved knowledge sources.
Do not create accounts, disclose credentials, order equipment or transmit medical/employment documents without authorization.
Never include passwords in messages."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
