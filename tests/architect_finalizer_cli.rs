use ocaf_runtime::{
    architect_research_evidence::{ResearchEvidence, ResearchEvidenceGate, SourceRecord},
    architect_research_schedule::{ArchitectResearchScheduler, ResearchKind, RunState},
    architect_workflow_bridge::register_due,
    om_engine::OmEngine,
    omnidev_workflow::{Outcome, Role},
};
use openssl::{
    pkey::{PKey, Private},
    sign::Signer,
};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    process::{Command, Output},
};
use tempfile::tempdir;

const NOW: i64 = 1_740_000_000;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn keys() -> [PKey<Private>; 3] {
    [
        PKey::generate_ed25519().unwrap(),
        PKey::generate_ed25519().unwrap(),
        PKey::generate_ed25519().unwrap(),
    ]
}
fn run(paths: [&Path; 3], k: &[PKey<Private>; 3], tenant: &str) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ocaf-architect-finalizer"));
    cmd.arg("--once")
        .env("OCAF_FINALIZER_SCHEDULE_DB", paths[0])
        .env("OCAF_FINALIZER_OM_DB", paths[1])
        .env("OCAF_FINALIZER_EVIDENCE_DB", paths[2])
        .env("OCAF_FINALIZER_TENANT", tenant)
        .env("OCAF_FINALIZER_FLEET", "fleet")
        .env(
            "OCAF_FINALIZER_PLANNER_PUBKEY_ED25519",
            hex(&k[0].raw_public_key().unwrap()),
        )
        .env(
            "OCAF_FINALIZER_ARCHITECT_PUBKEY_ED25519",
            hex(&k[1].raw_public_key().unwrap()),
        )
        .env(
            "OCAF_FINALIZER_VERIFIER_PUBKEY_ED25519",
            hex(&k[2].raw_public_key().unwrap()),
        );
    cmd.output().unwrap()
}
fn source(id: u8) -> SourceRecord {
    SourceRecord {
        url: format!("https://example.org/integration/{id}"),
        retrieved_unix: (NOW - 10) as u64,
        content_sha256: sha(&[id]),
    }
}
fn sign(k: &PKey<Private>, e: &ResearchEvidence) -> Vec<u8> {
    Signer::new_without_digest(k)
        .unwrap()
        .sign_oneshot_to_vec(&e.signing_message().unwrap())
        .unwrap()
}

#[test]
fn real_binary_finalizes_only_signed_research_and_skips_unfinished_periods() {
    let dir = tempdir().unwrap();
    let paths = [
        dir.path().join("schedule.db"),
        dir.path().join("om.db"),
        dir.path().join("evidence.db"),
    ];
    let scheduler = ArchitectResearchScheduler::open(&paths[0]).unwrap();
    let engine = OmEngine::open(&paths[1]).unwrap();
    let k = keys();
    let gate = ResearchEvidenceGate::open(
        &paths[2],
        &k[0].raw_public_key().unwrap(),
        &k[1].raw_public_key().unwrap(),
        &k[2].raw_public_key().unwrap(),
    )
    .unwrap();
    scheduler.tick("tenant", "fleet", NOW).unwrap();
    let entries = register_due(
        &scheduler,
        &engine,
        "tenant",
        "fleet",
        &"a".repeat(40),
        90,
        NOW,
        64,
    )
    .unwrap();
    let selected = entries
        .iter()
        .find(|r| r.kind == ResearchKind::Competitive)
        .unwrap();
    let mut research_sha = None;
    for (step, role, epoch, idx) in [
        ("plan", Role::Planner, 1, 0),
        ("research", Role::Architect, 2, 1),
        ("verify", Role::Verifier, 3, 2),
    ] {
        let attempt = format!("step_{epoch}");
        engine
            .claim(
                "tenant",
                &selected.workflow_id,
                role,
                step,
                epoch,
                &attempt,
                (NOW + epoch as i64) as u64,
            )
            .unwrap();
        let report = ResearchEvidence {
            schema_version: 1,
            tenant: "tenant".into(),
            workflow_id: selected.workflow_id.clone(),
            step: step.into(),
            role,
            epoch,
            attempt_id: attempt,
            observed_at_unix: (NOW + epoch as i64 + 1) as u64,
            actual_cost_minor: 5,
            outcome: Outcome::Passed,
            summary: "Signed fixture of deterministic verified pipeline, no actual web research"
                .into(),
            sources: if step == "research" {
                vec![source(1), source(2)]
            } else {
                vec![]
            },
            review_of_sha256: if step == "verify" {
                research_sha.clone()
            } else {
                None
            },
        };
        let receipt = gate
            .submit(
                &engine,
                "tenant",
                &report,
                &sign(&k[idx], &report),
                (NOW + epoch as i64 + 2) as u64,
            )
            .unwrap();
        if step == "research" {
            research_sha = Some(receipt.digest_sha256);
        }
    }
    let args = [paths[0].as_path(), paths[1].as_path(), paths[2].as_path()];
    // A process configured with different pinned signers must fail closed
    // and must not move the scheduler state.
    let untrusted = keys();
    let wrong_signer = run(args, &untrusted, "tenant");
    assert!(!wrong_signer.status.success());
    assert_eq!(
        scheduler
            .snapshot("tenant", "fleet", selected.kind, selected.period_start_unix)
            .unwrap()
            .state,
        RunState::Ready
    );

    // A malicious journal edit cannot convert a signed evidence record into
    // an accepted receipt, even if the OM Engine event chain is unchanged.
    let db = rusqlite::Connection::open(&paths[2]).unwrap();
    let original: String = db.query_row(
        "SELECT evidence_json FROM architect_research_evidence WHERE tenant='tenant' AND step='verify'",
        [], |r|r.get(0)
    ).unwrap();
    db.execute(
        "UPDATE architect_research_evidence SET evidence_json='{}' WHERE tenant='tenant' AND step='verify'",
        [],
    ).unwrap();
    let corrupted = run(args, &k, "tenant");
    assert!(!corrupted.status.success());
    assert_eq!(
        scheduler
            .snapshot("tenant", "fleet", selected.kind, selected.period_start_unix)
            .unwrap()
            .state,
        RunState::Ready
    );
    db.execute(
        "UPDATE architect_research_evidence SET evidence_json=?1 WHERE tenant='tenant' AND step='verify'",
        rusqlite::params![original],
    ).unwrap();
    drop(db);
    let result = run(args, &k, "tenant");
    assert!(
        result.status.success(),
        "{:?}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("scanned=3 finalized=1 pending=2"),
        "{:?}",
        String::from_utf8_lossy(&result.stdout),
    );
    assert_eq!(
        scheduler
            .snapshot("tenant", "fleet", selected.kind, selected.period_start_unix)
            .unwrap()
            .state,
        RunState::Succeeded
    );
    let repeated = run(args, &k, "tenant");
    assert!(repeated.status.success());
    assert!(String::from_utf8_lossy(&repeated.stdout).contains("scanned=2 finalized=0 pending=2"));
    // Alternate tenant cannot see signed study in a different tenant scope.
    let alternate = run(args, &k, "other");
    assert!(alternate.status.success());
    assert!(String::from_utf8_lossy(&alternate.stdout).contains("scanned=0 finalized=0 pending=0"));
}

#[test]
fn binary_rejects_invalid_keys_duplicate_journals_or_unknown_arguments() {
    let dir = tempdir().unwrap();
    let paths = [
        dir.path().join("schedule.db"),
        dir.path().join("om.db"),
        dir.path().join("evidence.db"),
    ];
    ArchitectResearchScheduler::open(&paths[0]).unwrap();
    OmEngine::open(&paths[1]).unwrap();
    let k = keys();
    ResearchEvidenceGate::open(
        &paths[2],
        &k[0].raw_public_key().unwrap(),
        &k[1].raw_public_key().unwrap(),
        &k[2].raw_public_key().unwrap(),
    )
    .unwrap();
    let same = [paths[0].as_path(), paths[0].as_path(), paths[2].as_path()];
    assert!(!run(same, &k, "tenant").status.success());
    let bad_tenant = run(
        [paths[0].as_path(), paths[1].as_path(), paths[2].as_path()],
        &k,
        "../escape",
    );
    assert!(!bad_tenant.status.success());
    let mut command = Command::new(env!("CARGO_BIN_EXE_ocaf-architect-finalizer"));
    command.arg("--not-supported");
    assert!(!command.output().unwrap().status.success());
    #[cfg(unix)]
    {
        // Two different paths may be hard links to the *same* SQLite file.
        // Journals must be physically distinct, not merely distinct strings.
        let hardlink = dir.path().join("hardlink.db");
        std::fs::hard_link(&paths[0], &hardlink).unwrap();
        let result = run(
            [paths[0].as_path(), hardlink.as_path(), paths[2].as_path()],
            &k,
            "tenant",
        );
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("journal files must be distinct"),
            "unexpected failure cause: {:?}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let alias = dir.path().join("alias.db");
        symlink(&paths[0], &alias).unwrap();
        assert!(!run(
            [alias.as_path(), paths[1].as_path(), paths[2].as_path()],
            &k,
            "tenant"
        )
        .status
        .success());
    }
}
