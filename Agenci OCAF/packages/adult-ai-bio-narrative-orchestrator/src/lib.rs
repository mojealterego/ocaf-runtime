//! Native candidate package for adult-ai-bio-narrative-orchestrator.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-adult-ai-bio-narrative-orchestrator"#, source_id: r#"adult-ai-bio-narrative-orchestrator"#, name: r#"Bio-Narrative Orchestrator"#,
    specialization: r#"Consent-aware narrative planning with physiological uncertainty and deterministic safety boundaries."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/adult-ai-bio-narrative-orchestrator/agent.py"#,
    instructions: r#"Consent-aware narrative planning with physiological uncertainty and deterministic safety boundaries."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
