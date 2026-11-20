//! DGM-inspired branching archive and bounded RSI incumbent selection.
//! Proposals are inert data. The host owns the proposer and evaluator.
//! Nothing in this module edits, builds, deploys or authorizes production code.

use crate::{AgentProfile, POLICY};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const MAX_ROUNDS: u32 = 32;
pub const MAX_PROPOSAL_BYTES: usize = 16_384;

#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub id: String,
    pub parent: Option<String>,
    pub round: u32,
    pub proposal: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Evaluation {
    pub score: u16,
    pub safety_passed: bool,
    pub regression_passed: bool,
}

impl Evaluation {
    fn admissible(self) -> bool {
        self.safety_passed && self.regression_passed
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveEntry {
    pub candidate: Candidate,
    pub evaluation: Evaluation,
}

#[derive(Debug, Serialize)]
pub struct SearchReport {
    pub agent_id: String,
    pub archive: Vec<ArchiveEntry>,
    pub incumbent: Candidate,
    pub incumbent_score: u16,
    pub release_admission: &'static str,
}

fn candidate(profile: &AgentProfile, parent: Option<String>, round: u32, proposal: String) -> Result<Candidate, String> {
    if proposal.trim().is_empty() || proposal.len() > MAX_PROPOSAL_BYTES {
        return Err("empty or oversized proposal".into());
    }
    let canonical = serde_json::to_vec(&(profile.id, profile.instructions, POLICY, &parent, round, &proposal))
        .map_err(|error| error.to_string())?;
    let id = format!("{:x}", Sha256::digest(canonical));
    Ok(Candidate { id, parent, round, proposal })
}

/// The caller supplies a proposer and a separately controlled evaluator. Scores
/// are not attestations; production promotion is intentionally not implemented here.
/// Every round records rejected as well as passing proposals. Odd rounds explore
/// prior admissible archive parents, even rounds refine the current incumbent.
pub fn search<P, E>(profile: &AgentProfile, rounds: u32, mut propose: P, mut evaluate: E) -> Result<SearchReport, String>
where
    P: FnMut(&Candidate, u32) -> Result<String, String>,
    E: FnMut(&Candidate) -> Result<Evaluation, String>,
{
    if rounds == 0 || rounds > MAX_ROUNDS {
        return Err("round budget outside 1..=32".into());
    }
    let root = candidate(profile, None, 0, "Baseline: unchanged agent behavior".into())?;
    let root_evaluation = evaluate(&root)?;
    if root_evaluation.score > 1000 || !root_evaluation.admissible() {
        return Err("baseline evaluation failed".into());
    }
    let mut incumbent = root.clone();
    let mut incumbent_score = root_evaluation.score;
    let mut archive = vec![ArchiveEntry { candidate: root, evaluation: root_evaluation }];
    for round in 1..=rounds {
        let parent = if round % 2 == 0 {
            incumbent.clone()
        } else {
            let eligible: Vec<_> = archive.iter().filter(|entry| entry.evaluation.admissible()).collect();
            eligible[((round / 2) as usize) % eligible.len()].candidate.clone()
        };
        let proposal = propose(&parent, round)?;
        let next = candidate(profile, Some(parent.id), round, proposal)?;
        let evaluation = evaluate(&next)?;
        if evaluation.score > 1000 {
            return Err("evaluator score outside 0..=1000".into());
        }
        if evaluation.admissible() && evaluation.score > incumbent_score {
            incumbent = next.clone();
            incumbent_score = evaluation.score;
        }
        archive.push(ArchiveEntry { candidate: next, evaluation });
    }
    Ok(SearchReport { agent_id: profile.id.into(), archive, incumbent, incumbent_score, release_admission: "DENY" })
}
