//! Native candidate package for devops-sre.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-devops-sre"#, source_id: r#"devops-sre"#, name: r#"Autonomous DevOps / SRE Agent"#,
    specialization: r#"Alert triage, incident correlation and bounded remediation proposals."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/occupational-ai/agent.py"#,
    instructions: r#"IT
Triage monitoring alerts, correlate logs and deployments, propose rollback/scaling options, and draft incident and post-mortem records.
No production commands, rollback, scaling, secret access or paging without explicit authorized approval.
Prioritize evidence preservation and reversible mitigation."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
