//! OCAF component acceptance gate.
//!
//! Detects incomplete agent, system, swarm, OMAt and OMAs directories.
//! This is structural evidence only; it does not certify executable behavior.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const FAMILIES: [(&str, &str); 5] = [
    ("agents", "agent"),
    ("systems", "system"),
    ("swarms", "swarm"),
    ("omat", "omat"),
    ("omas", "omas"),
];

#[derive(Debug, Default)]
pub struct Audit {
    pub inspected: usize,
    pub missing_targets: Vec<String>,
    pub errors: Vec<String>,
}

impl Audit {
    pub fn accepted(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn audit(repo: &Path, strict_targets: bool) -> io::Result<Audit> {
    let mut report = Audit::default();
    let registry = repo.join("ops/OCAF_REQUIRED_COMPONENTS.tsv");
    let inventory = fs::read_to_string(&registry)?;
    let mut required = BTreeSet::new();

    for (index, line) in inventory.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let columns: Vec<_> = line.split_whitespace().collect();
        if columns.len() != 2
            || !FAMILIES.iter().any(|(dir, _)| *dir == columns[0])
            || !safe_name(columns[1])
        {
            report.errors.push(format!(
                "registry:{}: invalid component family/name",
                index + 1
            ));
            continue;
        }
        if !required.insert(format!("{}/{}", columns[0], columns[1])) {
            report.errors.push(format!("registry:{}: duplicated component", index + 1));
        }
    }

    let mut found = BTreeSet::new();
    for (directory, expected_kind) in FAMILIES {
        let family_dir = repo.join("components").join(directory);
        if !family_dir.exists() {
            continue;
        }
        if is_symlink(&family_dir)? {
            report.errors.push(format!("{}: symlinked family is forbidden", directory));
            continue;
        }
        for entry in fs::read_dir(&family_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let key = format!("{directory}/{name}");
            if is_symlink(&path)? {
                report.errors.push(format!("{key}: symlink is forbidden"));
                continue;
            }
            if !path.is_dir() {
                report.errors.push(format!("{key}: expected a component directory"));
                continue;
            }
            found.insert(key.clone());
            report.inspected += 1;
            if !required.contains(&key) {
                report.errors.push(format!("{key}: component is not registered"));
            }
            validate_component(&path, expected_kind, &key, &mut report)?;
        }
    }

    for component in required.difference(&found) {
        report.missing_targets.push(component.clone());
        if strict_targets {
            report.errors.push(format!(
                "{component}: required implementation folder is missing"
            ));
        }
    }

    report.missing_targets.sort();
    report.errors.sort();
    Ok(report)
}

fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && !value.starts_with('.')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn is_symlink(path: &Path) -> io::Result<bool> {
    Ok(fs::symlink_metadata(path)?.file_type().is_symlink())
}

fn read_required(path: &Path, label: &str, errors: &mut Vec<String>) -> Option<String> {
    if !path.is_file() {
        errors.push(format!("{label}: missing file {}", path.display()));
        return None;
    }
    match fs::read_to_string(path) {
        Ok(content) if content.trim().len() >= 24 => Some(content),
        Ok(_) => {
            errors.push(format!("{label}: empty or placeholder file {}", path.display()));
            None
        }
        Err(err) => {
            errors.push(format!("{label}: cannot read {}: {err}", path.display()));
            None
        }
    }
}

fn manifest_value(contents: &str, key: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.split('#').next()?.trim();
        let (name, value) = match line.split_once('=') {
            Some(pair) => pair,
            None => continue,
        };
        if name.trim() == key {
            let value = value.trim();
            if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
                return Some(value[1..value.len() - 1].to_string());
            }
        }
    }
    None
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>, errors: &mut Vec<String>) -> io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if is_symlink(&path)? {
            errors.push(format!("symlinked source forbidden: {}", path.display()));
        } else if path.is_dir() {
            rust_sources(&path, out, errors)?;
        } else if path.extension().and_then(|x| x.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn validate_component(
    path: &Path,
    expected_kind: &str,
    key: &str,
    report: &mut Audit,
) -> io::Result<()> {
    let manifest = read_required(&path.join("component.toml"), key, &mut report.errors);
    let cargo = read_required(&path.join("Cargo.toml"), key, &mut report.errors);
    let kind = manifest.as_deref().and_then(|value| manifest_value(value, "kind"));
    let transport = manifest.as_deref().and_then(|value| manifest_value(value, "transport"));
    let maturity = manifest.as_deref().and_then(|value| manifest_value(value, "maturity"));

    if kind.as_deref() != Some(expected_kind) {
        report.errors.push(format!("{key}: incorrect or missing component kind"));
    }
    if !matches!(transport.as_deref(), Some("zenoh" | "local" | "none")) {
        report.errors.push(format!("{key}: transport must be zenoh, local or none"));
    }
    if matches!(expected_kind, "swarm" | "omas") && transport.as_deref() != Some("zenoh") {
        report.errors.push(format!("{key}: distributed swarm or OMAs must use Zenoh"));
    }
    if !matches!(maturity.as_deref(), Some("development" | "verified")) {
        report.errors.push(format!("{key}: maturity must be development or verified"));
    }
    if let Some(ref toml) = cargo {
        if !toml.contains("[package]") || !toml.lines().any(|line| line.trim_start().starts_with("name =")) {
            report.errors.push(format!("{key}: Cargo.toml lacks a Rust package"));
        }
        if transport.as_deref() == Some("zenoh")
            && !toml.lines().any(|line| line.trim_start().starts_with("zenoh ") || line.trim_start().starts_with("zenoh="))
        {
            report.errors.push(format!("{key}: Zenoh dependency missing from Cargo.toml"));
        }
    }

    let primary = path.join("src/lib.rs");
    let alternate = path.join("src/main.rs");
    if !primary.is_file() && !alternate.is_file() {
        report.errors.push(format!("{key}: missing src/lib.rs or src/main.rs"));
    }

    let mut sources = Vec::new();
    rust_sources(&path.join("src"), &mut sources, &mut report.errors)?;
    let mut code = String::new();
    for file in sources {
        if let Ok(contents) = fs::read_to_string(file) {
            code.push_str(&contents);
            code.push('\n');
        }
    }
    if code.len() < 100 || !code.contains("fn ") {
        report.errors.push(format!("{key}: missing substantive executable Rust code"));
    }
    if transport.as_deref() == Some("zenoh")
        && !code.lines().any(|line| line.trim_start().starts_with("use zenoh::") || line.trim_start().starts_with("zenoh::open("))
    {
        report.errors.push(format!("{key}: executable Rust source lacks Zenoh integration"));
    }

    let mut tests = Vec::new();
    rust_sources(&path.join("tests"), &mut tests, &mut report.errors)?;
    let mut tests_present = false;
    for path in tests {
        if let Ok(code) = fs::read_to_string(path) {
            if code.contains("#[test]") || code.contains("#[tokio::test]") {
                tests_present = true;
            }
        }
    }
    if !tests_present {
        report.errors.push(format!("{key}: missing executable Rust integration tests"));
    }

    for name in ["architecture.md", "user.md", "certification.md"] {
        read_required(&path.join("docs").join(name), key, &mut report.errors);
    }

    // Local evidence is not independent cryptographic certification. A verified
    // marker requires at least a concrete SHA/run-id receipt; release review
    // must additionally validate it against the provider and operator records.
    if maturity.as_deref() == Some("verified") {
        let receipt = read_required(&path.join("evidence/ci.json"), key, &mut report.errors);
        if let Some(receipt) = receipt {
            if !receipt.contains("commit_sha") || !receipt.contains("run_id") {
                report.errors.push(format!("{key}: verification receipt omits SHA or run ID"));
            }
        }
    }
    Ok(())
}
