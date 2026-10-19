//! Structural admission gate for real OCAF agents, systems, swarms, OMAt and OMAs.
//! This gate does not certify runtime behavior; full build, Zenoh and host evidence is separate.
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

const DOCS: &[&str] = &[
    "docs/ARCHITECTURE.md",
    "docs/TECHNICAL.md",
    "docs/USER_GUIDE.md",
    "docs/OPERATIONS.md",
    "docs/CERTIFICATION.md",
];

fn substantial(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|s| s.trim().len() >= 60)
        .unwrap_or(false)
}

fn has_rust_implementation(dir: &Path) -> bool {
    ["src/lib.rs", "src/main.rs"]
        .iter()
        .map(|p| dir.join(p))
        .any(|p| {
            fs::read_to_string(p).map(|s| {
                s.len() >= 100
                    && (s.contains("fn ") || s.contains("impl "))
                    && !s.contains("todo!()")
                    && !s.contains("unimplemented!()")
            }).unwrap_or(false)
        })
}

fn has_rust_tests(dir: &Path) -> bool {
    fs::read_dir(dir.join("tests"))
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            entry.path().extension().is_some_and(|e| e == "rs")
                && fs::read_to_string(entry.path())
                    .map(|s| s.contains("#[test]") || s.contains("#[tokio::test]"))
                    .unwrap_or(false)
        })
}

fn legal_component_path(relative: &Path) -> bool {
    !relative.is_absolute()
        && relative.components().all(|part| matches!(part, Component::Normal(_)))
}

fn audit(root: &Path, release: bool) -> (usize, Vec<String>) {
    let mut errors = Vec::new();
    let file = root.join("ocaf-components.tsv");
    let registry = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => return (0, vec![format!("Missing registry {}: {e}", file.display())]),
    };
    let mut seen_ids = HashSet::new();
    let mut seen_paths = HashSet::new();
    let mut count = 0_usize;
    for (index, line) in registry.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 4 {
            errors.push(format!("Registry line {} must contain 4 TAB-separated fields", index + 1));
            continue;
        }
        let (id, rel, kind, zenoh) = (fields[0], fields[1], fields[2], fields[3]);
        count += 1;
        if !seen_ids.insert(id.to_owned()) || !seen_paths.insert(rel.to_owned()) {
            errors.push(format!("Duplicate component id/path: {id} / {rel}"));
            continue;
        }
        if !matches!(kind, "agent" | "agent_system" | "swarm" | "omat" | "omas") {
            errors.push(format!("{id}: unknown component kind {kind}"));
            continue;
        }
        if !matches!(zenoh, "true" | "false") {
            errors.push(format!("{id}: zenoh_required must be true or false"));
            continue;
        }
        let relative = Path::new(rel);
        if !legal_component_path(relative)
            || !["agents/", "agent_systems/", "swarms/", "omat/", "omas/"]
                .iter()
                .any(|prefix| rel.starts_with(prefix))
        {
            errors.push(format!("{id}: invalid or out-of-scope folder {rel}"));
            continue;
        }
        let dir = root.join(relative);
        if !dir.is_dir() || dir.is_symlink() {
            errors.push(format!("{id}: MISSING dedicated source folder {}", dir.display()));
            continue;
        }
        if !substantial(&dir.join("Cargo.toml")) {
            errors.push(format!("{id}: missing/non-substantive Cargo.toml"));
        }
        if !has_rust_implementation(&dir) {
            errors.push(format!("{id}: missing executable Rust implementation in src/"));
        }
        if !has_rust_tests(&dir) {
            errors.push(format!("{id}: missing real Rust tests/*.rs"));
        }
        let metadata = fs::read_to_string(dir.join("component.toml")).unwrap_or_default();
        if !metadata.contains(&format!("id = \"{id}\""))
            || !metadata.contains(&format!("kind = \"{kind}\""))
            || !metadata.contains(&format!("zenoh_required = {zenoh}"))
        {
            errors.push(format!("{id}: component.toml disagrees with registry"));
        }
        for doc in DOCS {
            if !substantial(&dir.join(doc)) {
                errors.push(format!("{id}: missing/non-substantive {doc}"));
            }
        }
        if zenoh == "true" {
            let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
            if !cargo.contains("zenoh") {
                errors.push(format!("{id}: Zenoh required but absent from Cargo.toml"));
            }
            if !substantial(&dir.join("src/zenoh.rs")) {
                errors.push(format!("{id}: Zenoh required but src/zenoh.rs missing"));
            }
            if !substantial(&dir.join("tests/zenoh_transport.rs")) {
                errors.push(format!("{id}: Zenoh required but transport tests missing"));
            }
        }
        if release {
            let cert = fs::read_to_string(dir.join("component.toml")).unwrap_or_default();
            if !cert.contains("status = \"CERTIFIED\"")
                || !substantial(&dir.join("evidence/acceptance.json"))
            {
                errors.push(format!("{id}: NOT CERTIFIED — missing acceptance evidence"));
            }
        }
    }
    if count == 0 {
        errors.push("Registry contains no actual OCAF components".to_owned());
    }
    (count, errors)
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 2 || args.len() > 3 || (args.len() == 3 && args[2] != "--release") {
        eprintln!("Usage: ocaf-agent-artifact-gate REPOSITORY_ROOT [--release]");
        std::process::exit(2);
    }
    let (count, errors) = audit(&PathBuf::from(&args[1]), args.len() == 3);
    println!("OCAF structural artifact audit: {count} registered components");
    for issue in &errors {
        eprintln!("FAIL: {issue}");
    }
    if !errors.is_empty() {
        eprintln!("{} artifact contract violations; no certification inferred.", errors.len());
        std::process::exit(1);
    }
    println!("Structural gate passed; this does NOT replace cargo tests, Zenoh integration, CI or independent certification.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    fn fixture() -> PathBuf {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = env::temp_dir().join(format!("ocaf-artifact-contract-{}-{now}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }
    fn write(root: &Path, relative: &str, value: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, value).unwrap();
    }
    fn basic(root: &Path, with_zenoh: bool) {
        write(root, "ocaf-components.tsv", &format!("G01\tagents/G01\tagent\t{}\n", with_zenoh));
        write(root, "agents/G01/component.toml", &format!(
            "id = \"G01\"\nkind = \"agent\"\nzenoh_required = {}\nstatus = \"IN_DEVELOPMENT\"\n", with_zenoh));
        write(root, "agents/G01/Cargo.toml", "[package]\nname = \"ocaf-g01\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[dependencies]\nzenoh = \"1.10\"\n");
        write(root, "agents/G01/src/main.rs", "fn validate_stage(stage: &str) -> bool { !stage.is_empty() && stage.starts_with(\"0.\") }\nfn main() { assert!(validate_stage(\"0.42\")); }\n");
        write(root, "agents/G01/tests/stage.rs", "#[test]\nfn accepts_stage() { assert!(\"0.42\".starts_with(\"0.\")); }\n");
        for name in DOCS {
            write(root, &format!("agents/G01/{name}"), "OCAF testing artifact documentation: this file describes intended operational interfaces, test scope, acceptance evidence, limitations, and current deployment status for the agent. Not yet certified.");
        }
        if with_zenoh {
            write(root, "agents/G01/src/zenoh.rs", "pub async fn transport_contract(topic: &str) -> Result<(), &'static str> { if topic.starts_with(\"ocaf/\") { Ok(()) } else { Err(\"invalid topic\") } }\n");
            write(root, "agents/G01/tests/zenoh_transport.rs", "#[test]\nfn rejects_external_topics() { assert!(!\"external/tasks\".starts_with(\"ocaf/\")); }\n");
        }
    }
    #[test]
    fn rejects_markdown_only_agents() {
        let root = fixture();
        write(&root, "ocaf-components.tsv", "G01\tagents/G01\tagent\tfalse\n");
        write(&root, "agents/G01/docs/ARCHITECTURE.md", "A page of markdown without any implementation, tests, or executable Rust code should always be rejected.");
        let (_, errors) = audit(&root, false);
        assert!(errors.iter().any(|e| e.contains("Rust implementation")));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn complete_structure_passes_but_is_not_certification() {
        let root = fixture();
        basic(&root, false);
        assert!(audit(&root, false).1.is_empty());
        assert!(audit(&root, true).1.iter().any(|e| e.contains("NOT CERTIFIED")));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn missing_zenoh_transport_is_rejected() {
        let root = fixture();
        basic(&root, true);
        fs::remove_file(root.join("agents/G01/src/zenoh.rs")).unwrap();
        let (_, errors) = audit(&root, false);
        assert!(errors.iter().any(|e| e.contains("src/zenoh.rs")));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn duplicate_registration_is_rejected() {
        let root = fixture();
        basic(&root, false);
        write(&root, "ocaf-components.tsv", "G01\tagents/G01\tagent\tfalse\nG01\tagents/G01\tagent\tfalse\n");
        assert!(audit(&root, false).1.iter().any(|e| e.contains("Duplicate")));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn blocks_path_traversal() {
        assert!(!legal_component_path(Path::new("../outside")));
        assert!(!legal_component_path(Path::new("/tmp/external")));
    }
}