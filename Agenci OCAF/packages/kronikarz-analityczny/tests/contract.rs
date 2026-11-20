use ocaf_ext_kronikarz_analityczny::{profile, prepare, evolve};
use ocaf_import_core::evolution::Evaluation;
#[test]
fn identity_and_deny_contract() {
    assert_eq!(profile().id, "OCAF-AGENT-EXT-kronikarz-analityczny");
    let task = prepare("tenant_a", "req_1", "test").unwrap();
    assert_eq!(task.admission, "DENY");
    assert!(task.allowed_tools.is_empty());
    assert!(prepare("tenant/*", "req_1", "x").is_err());
    assert!(prepare("tenant_a", "req_1", "").is_err());
}
#[test]
fn bounded_evolution_cannot_release() {
    let report = evolve(2, |_, n| Ok(format!("proposal-{n}")), |c| Ok(Evaluation {
        score: 10 + c.round as u16, safety_passed: c.round != 2, regression_passed: true
    })).unwrap();
    assert_eq!(report.incumbent.round, 1);
    assert_eq!(report.archive.len(), 3);
    assert_eq!(report.release_admission, "DENY");
}
