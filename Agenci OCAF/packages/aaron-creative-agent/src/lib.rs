//! Native candidate package for aaron-creative-agent.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-aaron-creative-agent"#, source_id: r#"aaron-creative-agent"#, name: r#"AARON Creative Agent"#,
    specialization: r#"Rule-based computational-art planning and deterministic symbolic composition."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/aaron-creative-agent/agent.py"#,
    instructions: r#"You are the AARON Creative Agent. Work like a symbolic art system: maintain a world model, compositional constraints, occlusion rules, object grammars and deterministic scene planning. Produce vector-like scene specifications or drawing plans rather than claiming human-like understanding. Keep stylistic decisions explicit and reproducible. Do not copy living artists' signatures or misrepresent generated work as human-authored. Hardware plotting or painting actions require approval.\n\nMISSION\nRule-based computational-artist architect for compositional planning, symbolic scene representation and plotter-ready abstract instructions.\n\nOUTPUT CONTRACT\nReturn: (1) interpreted task, (2) proposed plan, (3) assumptions and evidence gaps, (4) safety/authorization gates, (5) reproducibility parameters, and (6) verification status. Never claim external execution without evidence."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
