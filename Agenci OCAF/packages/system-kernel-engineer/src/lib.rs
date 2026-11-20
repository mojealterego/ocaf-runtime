//! Native candidate package for system-kernel-engineer.
pub mod zenoh;
use ocaf_import_core::{AgentProfile, TaskDraft, evolution::{Candidate, Evaluation, SearchReport}};
pub const PROFILE: AgentProfile = AgentProfile {
    id: r#"OCAF-AGENT-EXT-system-kernel-engineer"#, source_id: r#"system-kernel-engineer"#, name: r#"System Kernel Engineer"#,
    specialization: r#"Low-level OS/kernel, Rust/no_std, C-to-Rust, GPU and hardware-interface engineering with verification gates."#, source_ref: r#"https://github.com/mojealterego/Ageni-AI-mojealterego/blob/a86626db5f40630809ecbb4c4110a3774ff16150/agents/system-kernel-engineer/agent.py"#,
    instructions: r#"Low-level OS/kernel, Rust/no_std, C-to-Rust, GPU and hardware-interface engineering with verification gates."#,
};
pub fn profile() -> AgentProfile { PROFILE }
pub fn prepare(tenant: &str, request: &str, input: &str) -> Result<TaskDraft, &'static str> {
    ocaf_import_core::prepare(&PROFILE, tenant, request, input)
}
pub fn evolve<P,E>(rounds: u32, propose: P, evaluate: E) -> Result<SearchReport, String>
where P: FnMut(&Candidate,u32)->Result<String,String>, E: FnMut(&Candidate)->Result<Evaluation,String> {
    ocaf_import_core::evolution::search(&PROFILE, rounds, propose, evaluate)
}
