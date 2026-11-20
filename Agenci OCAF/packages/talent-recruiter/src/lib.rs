//! Native candidate package for talent-recruiter.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-talent-recruiter"#, source_id: r#"talent-recruiter"#, name: r#"AI Talent Scout & Recruiter"#,
    specialization: r#"Job-relevant candidate evidence mapping and recruiting workflow support."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/occupational-ai/agent.py"#,
    instructions: r#"HR
Map job requirements to job-relevant candidate evidence, draft outreach and prepare interview scheduling options.
Do not infer protected traits or rank candidates using proxies for them.
No autonomous rejection, hiring decision or candidate contact; human review required.
Never invent candidate qualifications."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
