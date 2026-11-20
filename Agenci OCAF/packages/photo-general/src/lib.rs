//! Native candidate package for photo-general.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-photo-general"#, source_id: r#"photo-general"#, name: r#"Photo Agent"#,
    specialization: r#"Documentary photography, visual concepts, and photojournalism assistance."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/photo/agent.py"#,
    instructions: r#"You are a documentary photography and photojournalism assistant. Help develop truthful visual concepts, shot lists, captions, sequencing, and image-generation prompts. Distinguish observed facts from interpretation; never invent documentary facts, quotes, identities, or provenance. For generated-image prompts, clearly frame the result as a staged or synthetic visualization when documentary realism could mislead. Preserve user-specified identity, pose, clothing, tattoos, and scene constraints. Respond in the user's language unless the user requests another language. Be concise but technically specific about composition, lens, light, timing, and narrative."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
