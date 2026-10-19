use ocaf_component_gate::audit;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct TemporaryRepo(PathBuf);

impl TemporaryRepo {
    fn new() -> Self {
        let ticks = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "ocaf-gate-test-{}-{ticks}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, relative: &str, contents: &str) {
        let target = self.0.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, contents).unwrap();
    }

    fn register(&self, line: &str) {
        self.write("ops/OCAF_REQUIRED_COMPONENTS.tsv", line);
    }

    fn valid_component(&self, family: &str, name: &str, transport: &str) {
        let root = format!("components/{family}/{name}");
        let kind = match family {
            "agents" => "agent",
            "systems" => "system",
            "swarms" => "swarm",
            "omat" => "omat",
            "omas" => "omas",
            _ => panic!("unknown family"),
        };
        self.write(
            &format!("{root}/component.toml"),
            &format!("kind = \"{kind}\"\ntransport = \"{transport}\"\nmaturity = \"development\"\n"),
        );
        self.write(
            &format!("{root}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{}",
                if transport == "zenoh" { "zenoh = \"1\"\n" } else { "" }
            ),
        );
        self.write(
            &format!("{root}/src/lib.rs"),
            if transport == "zenoh" {
                "use zenoh::Session;\npub fn validate_request(value: &str) -> bool { let _zenoh_integration: Option<Session> = None; !value.is_empty() && value.len() < 1024 }\n"
            } else {
                "pub fn validate_request(value: &str) -> bool { !value.is_empty() && value.len() < 1024 }\n// Validate only bounded, non-empty externally supplied request bodies.\n"
            },
        );
        self.write(
            &format!("{root}/tests/integration.rs"),
            "#[test]\nfn valid_requests_are_accepted() { assert_eq!(2 + 2, 4); }\n",
        );
        for document in ["architecture.md", "user.md", "certification.md"] {
            self.write(
                &format!("{root}/docs/{document}"),
                "This document describes the component, its boundaries and the required verification evidence. No production certification is claimed.\n",
            );
        }
    }
}

impl Drop for TemporaryRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn rejects_markdown_only_component() {
    let repo = TemporaryRepo::new();
    repo.register("agents doc-only\n");
    repo.write(
        "components/agents/doc-only/docs/architecture.md",
        "An attractive but nonexecutable Markdown-only agent description is insufficient.",
    );
    let findings = audit(Path::new(&repo.0), false).unwrap();
    assert!(!findings.accepted());
    assert!(findings.errors.iter().any(|x| x.contains("Rust")));
}

#[test]
fn accepts_structurally_complete_local_agent() {
    let repo = TemporaryRepo::new();
    repo.register("agents example-agent\n");
    repo.valid_component("agents", "example-agent", "local");
    let findings = audit(&repo.0, true).unwrap();
    assert!(findings.accepted(), "{:?}", findings.errors);
    assert_eq!(findings.inspected, 1);
}

#[test]
fn refuses_non_zenoh_swarm() {
    let repo = TemporaryRepo::new();
    repo.register("swarms g01\n");
    repo.valid_component("swarms", "g01", "local");
    let findings = audit(&repo.0, true).unwrap();
    assert!(findings.errors.iter().any(|x| x.contains("must use Zenoh")));
}

#[test]
fn strict_mode_rejects_missing_real_group_folder() {
    let repo = TemporaryRepo::new();
    repo.register("swarms g01\n");
    let transitional = audit(&repo.0, false).unwrap();
    assert!(transitional.accepted());
    assert_eq!(transitional.missing_targets, vec!["swarms/g01"]);
    let strict = audit(&repo.0, true).unwrap();
    assert!(!strict.accepted());
}

#[test]
fn verified_component_requires_ci_receipt() {
    let repo = TemporaryRepo::new();
    repo.register("agents example-agent\n");
    repo.valid_component("agents", "example-agent", "local");
    repo.write(
        "components/agents/example-agent/component.toml",
        "kind = \"agent\"\ntransport = \"local\"\nmaturity = \"verified\"\n",
    );
    let findings = audit(&repo.0, true).unwrap();
    assert!(findings.errors.iter().any(|x| x.contains("evidence/ci.json")));
}
