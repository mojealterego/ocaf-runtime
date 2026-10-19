use ocaf_artifact_gate::{validate_tree, validate_unit};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("ocaf-artifact-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self) -> &Path { &self.0 }
    fn write(&self, name: &str, content: &str) {
        let p = self.0.join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }
    fn make_agent(&self) {
        self.write("agents/worker/ocaf-artifact.contract", "format = \"ocaf-artifact-v1\"\nname = \"worker\"\nkind = \"agent\"\ntransport = \"none\"\n");
        self.write("agents/worker/Cargo.toml", "[package]\nname = \"worker\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
        self.write("agents/worker/src/lib.rs", "pub fn run() -> bool { true }\n");
        self.write("agents/worker/tests/check.rs", "#[test] fn run() { assert!(true); }\n");
        for doc in ["ARCHITECTURE.md", "USER_GUIDE.md", "CERTIFICATION.md", "TRANSPORT.md"] {
            self.write(&format!("agents/worker/docs/{doc}"), &"Reproducible test plan and operator evidence are required before production status. ".repeat(3));
        }
    }
}
impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }
#[test]
fn valid_rust_artifact_is_structurally_admitted() {
    let f = Fixture::new(); f.make_agent();
    assert!(validate_tree(f.path()).is_empty());
}
#[test]
fn markdown_only_folder_is_rejected() {
    let f = Fixture::new(); f.write("agents/notes/README.md", "Documentation only");
    assert!(validate_tree(f.path()).iter().any(|s| s.contains("contract")));
}
#[test]
fn missing_rust_tests_is_rejected() {
    let f = Fixture::new(); f.make_agent();
    fs::remove_file(f.path().join("agents/worker/tests/check.rs")).unwrap();
    assert!(validate_unit(&f.path().join("agents/worker"), "agent")
        .iter().any(|e| e.contains("Rust regression tests")));
}
#[test]
fn zenoh_cannot_be_declared_without_rust_transport_code() {
    let f = Fixture::new(); f.make_agent();
    f.write("agents/worker/ocaf-artifact.contract", "format = \"ocaf-artifact-v1\"\nname = \"worker\"\nkind = \"agent\"\ntransport = \"zenoh\"\n");
    assert!(validate_tree(f.path()).iter().any(|s| s.contains("Zenoh")));
}
#[test]
fn distinct_folder_identity_and_type_are_enforced() {
    let f = Fixture::new(); f.make_agent();
    assert!(validate_unit(&f.path().join("agents/worker"), "omas")
        .iter().any(|s| s.contains("kind")));
}
