use ocaf_import_core::{AgentProfile, prepare, evolution::*};

const PROFILE: AgentProfile = AgentProfile {
    id: "OCAF-AGENT-EXT-test", source_id: "test", name: "Test",
    specialization: "Evidence analysis", source_ref: "pinned-source",
    instructions: "Trace evidence; do not invent results.",
};

#[test]
fn draft_requests_are_bounded_and_do_not_grant_tools() {
    let task = prepare(&PROFILE, "tenant_a", "req_1", "Analyze evidence").unwrap();
    assert_eq!(task.admission, "DENY");
    assert!(task.allowed_tools.is_empty());
    assert_eq!(task.profile.id, PROFILE.id);
    assert!(prepare(&PROFILE, "tenant/*", "req_1", "x").is_err());
    assert!(prepare(&PROFILE, "tenant_a", "req_1", " ").is_err());
    assert!(prepare(&PROFILE, "tenant_a", "req_1", &"x".repeat(65537)).is_err());
}

#[test]
fn evolution_archives_branches_and_preserves_strict_rsi_incumbent() {
    let report = search(&PROFILE, 6, |parent, round| {
        Ok(format!("proposal-{round}-from-{}", parent.id))
    }, |candidate| {
        let score = if candidate.parent.is_none() { 10 } else { 20 + candidate.round as u16 };
        Ok(Evaluation { score, safety_passed: candidate.round != 3, regression_passed: true })
    }).unwrap();
    assert_eq!(report.archive.len(), 7);
    assert_eq!(report.incumbent.round, 6);
    assert!(report.archive.iter().any(|entry| !entry.evaluation.safety_passed));
    assert!(report.archive.iter().any(|entry| entry.candidate.parent.as_deref() == Some(report.archive[0].candidate.id.as_str())));
    assert_eq!(report.release_admission, "DENY");
    assert!(search(&PROFILE, 33, |_, _| Ok("x".into()), |_| Ok(Evaluation {score:1,safety_passed:true,regression_passed:true})).is_err());
}

#[test]
fn rsi_cannot_replace_incumbent_with_regression() {
    let report = search(&PROFILE, 2, |_, round| Ok(format!("candidate-{round}")), |candidate| {
        Ok(Evaluation { score: if candidate.parent.is_none() { 100 } else { 900 },
            safety_passed: true, regression_passed: candidate.parent.is_none() })
    }).unwrap();
    assert_eq!(report.incumbent.round, 0);
    assert_eq!(report.release_admission, "DENY");
}
