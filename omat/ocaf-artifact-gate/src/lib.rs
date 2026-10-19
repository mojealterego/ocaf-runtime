//! Structural Rust admission gate for OCAF agents, systems, swarms, OMAt and OMAs.
//! This check cannot certify actual operation, transport behavior or release approval.
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const SPACES: &[(&str, &str)] = &[
    ("agents", "agent"), ("agent_systems", "agent-system"),
    ("swarms", "swarm"), ("omat", "omat"), ("omas", "omas"),
];

fn regular(path: &Path) -> bool {
    fs::symlink_metadata(path).map(|m| m.file_type().is_file()).unwrap_or(false)
}

fn required_text(path: &Path, errors: &mut Vec<String>) -> Option<String> {
    if !regular(path) {
        errors.push(format!("{}: mandatory regular file missing; symlinks forbidden", path.display()));
        return None;
    }
    match fs::read_to_string(path) {
        Ok(s) => Some(s),
        Err(e) => { errors.push(format!("{}: {e}", path.display())); None }
    }
}

pub fn validate_unit(path: &Path, expected_kind: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if !path.is_dir() {
        return vec![format!("{}: dedicated artifact directory missing", path.display())];
    }
    let contract_path = path.join("ocaf-artifact.contract");
    let Some(contract) = required_text(&contract_path, &mut errors) else { return errors };
    let mut fields = BTreeMap::new();
    for (n, raw) in contract.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let Some((k, v)) = line.split_once('=') else {
            errors.push(format!("{}:{}: invalid contract syntax", contract_path.display(), n + 1));
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        if !["format", "name", "kind", "transport"].contains(&k) {
            errors.push(format!("{}:{}: unrecognized key {k}", contract_path.display(), n + 1));
            continue;
        }
        let Some(value) = v.strip_prefix('"').and_then(|x| x.strip_suffix('"')) else {
            errors.push(format!("{}:{}: quoted value required", contract_path.display(), n + 1));
            continue;
        };
        if value.is_empty() || fields.insert(k.to_owned(), value.to_owned()).is_some() {
            errors.push(format!("{}:{}: duplicate or empty field {k}", contract_path.display(), n + 1));
        }
    }
    if fields.get("format").map(String::as_str) != Some("ocaf-artifact-v1") {
        errors.push(format!("{}: invalid format", contract_path.display()));
    }
    if fields.get("kind").map(String::as_str) != Some(expected_kind) {
        errors.push(format!("{}: incorrect artifact kind", contract_path.display()));
    }
    let dirname = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
    if fields.get("name").map(String::as_str) != Some(dirname) {
        errors.push(format!("{}: name must match dedicated directory", contract_path.display()));
    }
    let cargo_path = path.join("Cargo.toml");
    let cargo = required_text(&cargo_path, &mut errors).unwrap_or_default();
    if !cargo.contains("[package]") || !cargo.contains("edition") || !cargo.contains("name") {
        errors.push(format!("{}: invalid Rust crate metadata", cargo_path.display()));
    }
    let has_code = ["src/lib.rs", "src/main.rs"].iter().any(|file| {
        let file = path.join(file);
        if !regular(&file) { return false; }
        fs::read_to_string(&file).ok().is_some_and(|source| {
            source.lines().any(|line| {
                let line = line.trim();
                !line.starts_with("//") &&
                    (line.contains("fn ") || line.contains("struct ") ||
                     line.contains("impl ") || line.contains("enum "))
            })
        })
    });
    if !has_code {
        errors.push(format!("{}: Rust implementation in src/lib.rs or src/main.rs is mandatory", path.display()));
    }
    let tests = path.join("tests");
    let has_tests = fs::read_dir(&tests).ok().is_some_and(|entries| {
        entries.flatten().any(|entry| {
            let file = entry.path();
            file.extension().and_then(|v| v.to_str()) == Some("rs")
                && regular(&file)
                && fs::read_to_string(&file).ok()
                    .is_some_and(|code| code.contains("#[test]") || code.contains("#[tokio::test]"))
        })
    });
    if !has_tests {
        errors.push(format!("{}: executable Rust regression tests in tests/ are mandatory", path.display()));
    }
    for doc in ["ARCHITECTURE.md", "USER_GUIDE.md", "CERTIFICATION.md"] {
        let file = path.join("docs").join(doc);
        if let Some(text) = required_text(&file, &mut errors) {
            if text.trim().len() < 80 {
                errors.push(format!("{}: documentation incomplete", file.display()));
            }
        }
    }
    match fields.get("transport").map(String::as_str) {
        Some("zenoh") => {
            if !cargo.contains("zenoh") || !regular(&path.join("src/zenoh.rs")) {
                errors.push(format!("{}: Zenoh dependency and src/zenoh.rs implementation required", path.display()));
            }
        }
        Some("none") => {
            let doc = path.join("docs/TRANSPORT.md");
            if let Some(text) = required_text(&doc, &mut errors) {
                if text.trim().len() < 80 {
                    errors.push(format!("{}: justify non-Zenoh transport", doc.display()));
                }
            }
        }
        _ => errors.push(format!("{}: transport must be zenoh or none", contract_path.display())),
    }
    errors
}

pub fn validate_tree(repo: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let mut count = 0;
    for (namespace, kind) in SPACES {
        let root = repo.join(namespace);
        if !root.exists() { continue; }
        let Ok(entries) = fs::read_dir(&root) else {
            errors.push(format!("{}: cannot list directory", root.display()));
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                count += 1;
                errors.extend(validate_unit(&p, kind));
            } else if p.file_name().and_then(|v| v.to_str()) != Some("README.md") {
                errors.push(format!("{}: artifact must be a directory", p.display()));
            }
        }
    }
    if count == 0 {
        errors.push("No Rust OCAF artifact folders found. Markdown alone is not implementation.".into());
    }
    errors
}
