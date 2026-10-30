//! PDF-1.50: fail-closed, read-only integrity verification for MCP skill registries.
//! This component never executes, installs or authorizes MCP tools.
//! omega/SHA256SUMS.txt paths are relative to omega/ and pair SKILL.md with
//! references/registry.md, NOT with a nonexistent reference.json.
//! The manifest SHA-256 pin MUST be supplied from an independent trusted source.
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const UPSTREAM_REGISTRY_COUNT: usize = 210;
const MAX_MANIFEST_SIZE: usize = 512 * 1024;
const MAX_FILE_SIZE: usize = 256 * 1024;
const MAX_TOTAL_SIZE: usize = 32 * 1024 * 1024;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum RegistryError {
    #[error("untrusted SHA256SUMS manifest")]
    InvalidManifestPin,
    #[error("invalid registry manifest path or hash encoding")]
    MalformedManifest,
    #[error("missing or duplicate registry pair")]
    InvalidRegistryPairs,
    #[error("missing, extra or duplicate supplied registry file")]
    InvalidInventory,
    #[error("provided content does not match pinned registry manifest")]
    HashMismatch,
    #[error("bounded registry resources exceeded")]
    Limit,
}

#[derive(Debug)]
pub struct RegistryFile<'a> {
    pub path: &'a str,
    pub content: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryReport {
    pub skills_verified: usize,
    pub files_verified: usize,
    pub manifest_sha256: [u8; 32],
    /// Independently checking tool operation requires a separate E2E gate.
    pub tools_executed: bool,
}

fn skill_path(path: &str) -> Option<(&str, bool)> {
    let segments: Vec<&str> = path.split('/').collect();
    let (skill, is_skill) = match segments.as_slice() {
        ["skills", skill, "SKILL.md"] => (*skill, true),
        ["skills", skill, "references", "registry.md"] => (*skill, false),
        _ => return None,
    };
    let stem = skill.strip_suffix("-mcp-registry")?;
    if stem.is_empty()
        || skill.len() > 96
        || stem.starts_with('-')
        || stem.ends_with('-')
        || !stem
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return None;
    }
    Some((skill, is_skill))
}

fn hex_digest(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut digest = [0u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(digest)
}

/// Verify a complete registry against a manifest whose SHA-256 was pinned
/// independently of both supplied manifest and files. Other SHA256SUMS entries
/// outside the registry are permitted, but unexpected registry paths fail.
/// This API checks bytes; it does not dereference filesystem symlinks.
pub fn verify_registry(
    manifest: &str,
    operator_pinned_sha256: [u8; 32],
    files: &[RegistryFile<'_>],
    expected_skills: usize,
) -> Result<RegistryReport, RegistryError> {
    if manifest.is_empty()
        || manifest.len() > MAX_MANIFEST_SIZE
        || expected_skills == 0
        || expected_skills > UPSTREAM_REGISTRY_COUNT
        || files.len() > UPSTREAM_REGISTRY_COUNT * 2
    {
        return Err(RegistryError::Limit);
    }
    let digest: [u8; 32] = Sha256::digest(manifest.as_bytes()).into();
    if digest != operator_pinned_sha256 {
        return Err(RegistryError::InvalidManifestPin);
    }
    let mut expected = BTreeMap::new();
    let mut pairs: BTreeMap<&str, (bool, bool)> = BTreeMap::new();
    for line in manifest.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_ascii_whitespace().collect();
        if parts.len() != 2 {
            return Err(RegistryError::MalformedManifest);
        }
        let hash = hex_digest(parts[0]).ok_or(RegistryError::MalformedManifest)?;
        let path = parts[1];
        let Some((skill, primary)) = skill_path(path) else {
            if path.starts_with("skills/") && path.contains("-mcp-registry/") {
                return Err(RegistryError::MalformedManifest);
            }
            continue;
        };
        if expected.insert(path, hash).is_some() {
            return Err(RegistryError::InvalidRegistryPairs);
        }
        let entry = pairs.entry(skill).or_insert((false, false));
        if primary {
            entry.0 = true;
        } else {
            entry.1 = true;
        }
    }
    if pairs.len() != expected_skills
        || expected.len() != expected_skills * 2
        || pairs.values().any(|pair| !pair.0 || !pair.1)
    {
        return Err(RegistryError::InvalidRegistryPairs);
    }
    if files.len() != expected.len() {
        return Err(RegistryError::InvalidInventory);
    }
    let mut seen = BTreeSet::new();
    let mut total = 0usize;
    for file in files {
        if skill_path(file.path).is_none() || !seen.insert(file.path) {
            return Err(RegistryError::InvalidInventory);
        }
        if file.content.is_empty() || file.content.len() > MAX_FILE_SIZE {
            return Err(RegistryError::Limit);
        }
        total = total
            .checked_add(file.content.len())
            .ok_or(RegistryError::Limit)?;
        if total > MAX_TOTAL_SIZE {
            return Err(RegistryError::Limit);
        }
        let expected_hash = expected
            .get(file.path)
            .ok_or(RegistryError::InvalidInventory)?;
        let actual_hash: [u8; 32] = Sha256::digest(file.content).into();
        if &actual_hash != expected_hash {
            return Err(RegistryError::HashMismatch);
        }
    }
    Ok(RegistryReport {
        skills_verified: expected_skills,
        files_verified: expected.len(),
        manifest_sha256: digest,
        tools_executed: false,
    })
}

pub fn verify_upstream_registry(
    manifest: &str,
    trusted_sha256: [u8; 32],
    files: &[RegistryFile<'_>],
) -> Result<RegistryReport, RegistryError> {
    verify_registry(manifest, trusted_sha256, files, UPSTREAM_REGISTRY_COUNT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (String, Vec<RegistryFile<'static>>) {
        let files = vec![
            RegistryFile {
                path: "skills/a-mcp-registry/SKILL.md",
                content: b"a skill",
            },
            RegistryFile {
                path: "skills/a-mcp-registry/references/registry.md",
                content: b"a reference",
            },
            RegistryFile {
                path: "skills/b-mcp-registry/SKILL.md",
                content: b"b skill",
            },
            RegistryFile {
                path: "skills/b-mcp-registry/references/registry.md",
                content: b"b reference",
            },
        ];
        let manifest = files
            .iter()
            .map(|f| format!("{:x}  {}", Sha256::digest(f.content), f.path))
            .collect::<Vec<_>>()
            .join("\n");
        (manifest, files)
    }

    fn pin(manifest: &str) -> [u8; 32] {
        Sha256::digest(manifest.as_bytes()).into()
    }

    #[test]
    fn accepts_complete_pairs_without_claiming_tools_were_run() {
        let (manifest, files) = fixture();
        let result = verify_registry(&manifest, pin(&manifest), &files, 2).unwrap();
        assert_eq!((result.skills_verified, result.files_verified), (2, 4));
        assert!(!result.tools_executed);
    }

    #[test]
    fn rejects_manifest_rewritten_after_pin() {
        let (manifest, files) = fixture();
        let mut altered = manifest.clone();
        altered.push('\n');
        assert_eq!(
            verify_registry(&altered, pin(&manifest), &files, 2).unwrap_err(),
            RegistryError::InvalidManifestPin
        );
    }

    #[test]
    fn rejects_tampered_skill_content() {
        let (manifest, mut files) = fixture();
        files[0].content = b"evil skill";
        assert_eq!(
            verify_registry(&manifest, pin(&manifest), &files, 2).unwrap_err(),
            RegistryError::HashMismatch
        );
    }

    #[test]
    fn rejects_missing_registry_file() {
        let (manifest, files) = fixture();
        assert_eq!(
            verify_registry(&manifest, pin(&manifest), &files[..3], 2).unwrap_err(),
            RegistryError::InvalidInventory
        );
    }

    #[test]
    fn rejects_duplicate_and_traversal_paths_even_if_rehashed() {
        let (manifest, files) = fixture();
        let duplicate = manifest.replace(
            "skills/b-mcp-registry/SKILL.md",
            "skills/a-mcp-registry/SKILL.md",
        );
        assert_eq!(
            verify_registry(&duplicate, pin(&duplicate), &files, 2).unwrap_err(),
            RegistryError::InvalidRegistryPairs
        );
        let traversal = manifest.replace(
            "skills/b-mcp-registry/SKILL.md",
            "skills/../b-mcp-registry/SKILL.md",
        );
        assert_eq!(
            verify_registry(&traversal, pin(&traversal), &files, 2).unwrap_err(),
            RegistryError::MalformedManifest
        );
    }

    #[test]
    fn rejects_legacy_reference_json_and_python_bytecode_entries() {
        let (manifest, files) = fixture();
        for bad in [
            "skills/b-mcp-registry/reference.json",
            "skills/b-mcp-registry/__pycache__/run.pyc",
        ] {
            let modified = manifest.replace("skills/b-mcp-registry/references/registry.md", bad);
            assert_eq!(
                verify_registry(&modified, pin(&modified), &files, 2).unwrap_err(),
                RegistryError::MalformedManifest
            );
        }
    }

    #[test]
    fn checks_exact_expected_skill_count() {
        let (manifest, files) = fixture();
        assert_eq!(
            verify_upstream_registry(&manifest, pin(&manifest), &files).unwrap_err(),
            RegistryError::InvalidRegistryPairs
        );
    }

    #[test]
    fn unrelated_manifest_entries_may_coexist_but_registry_paths_cannot_escape() {
        let (manifest, files) = fixture();
        let larger = format!("{manifest}\n{:x}  README.md", Sha256::digest(b"other"));
        assert_eq!(
            verify_registry(&larger, pin(&larger), &files, 2)
                .unwrap()
                .files_verified,
            4
        );
        assert!(skill_path("skills/a-mcp-registry/../SKILL.md").is_none());
    }
}
