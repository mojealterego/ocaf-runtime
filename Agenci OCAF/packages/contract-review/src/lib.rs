//! Native candidate package for contract-review.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-contract-review"#, source_id: r#"contract-review"#, name: r#"Contract Review & Redlining Agent"#,
    specialization: r#"Clause evidence extraction, playbook comparison and redline drafting."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/occupational-ai/agent.py"#,
    instructions: r#"Legal
Classify supplied contracts, extract clause-level evidence, compare against a supplied playbook, draft proposed redlines and summarize risks for counsel.
Not a substitute for qualified counsel; jurisdiction and playbook version must be explicit.
Never claim edits were applied to a Word document unless a connected tool confirms it.
Risk labels are preliminary and require legal review."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
