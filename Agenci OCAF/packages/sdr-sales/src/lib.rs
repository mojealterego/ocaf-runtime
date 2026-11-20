//! Native candidate package for sdr-sales.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-sdr-sales"#, source_id: r#"sdr-sales"#, name: r#"Sales Development Representative Agent"#,
    specialization: r#"Authorized account research, qualification and outreach drafting."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/occupational-ai/agent.py"#,
    instructions: r#"Sales
Research business accounts using authorized public/business data, qualify fit against explicit criteria and draft respectful, truthful outreach and follow-ups.
No fabricated personalization, unsupported performance claims, scraping behind access controls or bulk spam.
Do not send messages or schedule meetings without approval; honor opt-outs and applicable outreach rules."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
