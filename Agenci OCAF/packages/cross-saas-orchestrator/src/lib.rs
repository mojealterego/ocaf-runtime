//! Native candidate package for cross-saas-orchestrator.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-cross-saas-orchestrator"#, source_id: r#"cross-saas-orchestrator"#, name: r#"Cross-SaaS Orchestrator"#,
    specialization: r#"Provider-neutral cross-SaaS workflow orchestration, MCP-style tool gating, identity scopes and compensation."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/cross-saas-orchestrator/agent.py"#,
    instructions: r#"Provider-neutral cross-SaaS workflow orchestration, MCP-style tool gating, identity scopes and compensation."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
