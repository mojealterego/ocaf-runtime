//! Linux native static compiler and explicit local artifact export.
#[path = "../../../../src/builder_core.rs"]
mod canonical_builder_core;

use canonical_builder_core::{compile, from_dsl, to_dsl};
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::Write,
    os::unix::fs::MetadataExt,
    path::Path,
    sync::Mutex,
};
use tauri::{Manager, WebviewWindow};

static EXPORT_LOCK: Mutex<()> = Mutex::new(());

fn compile_local(source: &str) -> Result<Value, &'static str> {
    let project = from_dsl(source, "local").map_err(|_| "Rust validation rejected")?;
    let plan = compile(&project, "local").map_err(|_| "Rust validation rejected")?;
    Ok(json!({
        "namespace": "local",
        "project_sha256": plan.project_sha256,
        "ordered_nodes": plan.ordered_nodes,
        "requested_tools": plan.requested_tools,
        "max_cost_minor": plan.max_cost_minor,
        "effects_executed": false
    }))
}

// An exported Cloud draft is local data, not a grant of Cloud privileges.
// Validate its declared namespace and do not make any network requests.
fn export_to(directory: &Path, source: &str, tenant: &str) -> Result<String, &'static str> {
    let project = from_dsl(source, tenant).map_err(|_| "Rust export validation rejected")?;
    if !super::native_cloud::identifier(&project.project_id) {
        return Err("Invalid export identifier");
    }
    let canonical = to_dsl(&project, tenant).map_err(|_| "Rust export validation rejected")?;
    let meta = fs::metadata(directory).map_err(|_| "Downloads directory unavailable")?;
    if !directory.is_absolute()
        || directory.canonicalize().ok().as_deref() != Some(directory)
        || !meta.is_dir()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o022 != 0
    {
        return Err("Downloads directory must be canonical and user-owned");
    }
    let mut temporary = tempfile::Builder::new()
        .prefix(".ocaf-export-")
        .tempfile_in(directory)
        .map_err(|_| "Cannot create private export")?;
    temporary
        .write_all(canonical.as_bytes())
        .map_err(|_| "Cannot write export")?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| "Cannot sync export")?;
    for suffix in 0..100 {
        let stem = format!("{}-r{}", project.project_id, project.revision);
        let name = if suffix == 0 {
            format!("{stem}.ocaf-builder.json")
        } else {
            format!("{stem}-{suffix}.ocaf-builder.json")
        };
        match temporary.persist_noclobber(directory.join(&name)) {
            Ok(_) => {
                File::open(directory)
                    .and_then(|f| f.sync_all())
                    .map_err(|_| "Export written but directory sync failed")?;
                return Ok(name);
            }
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                temporary = error.file
            }
            Err(_) => return Err("Cannot publish export"),
        }
    }
    Err("Too many exports for this revision")
}

#[tauri::command]
pub fn native_local_compile(window: WebviewWindow, source: String) -> Result<Value, String> {
    super::native_cloud::require_caller(&window)?;
    compile_local(&source).map_err(str::to_owned)
}

#[tauri::command]
pub async fn native_export_project(
    window: WebviewWindow,
    source: String,
    tenant: String,
) -> Result<String, String> {
    super::native_cloud::require_caller(&window)?;
    let directory = window
        .app_handle()
        .path()
        .download_dir()
        .map_err(|_| "Downloads directory unavailable".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = EXPORT_LOCK
            .try_lock()
            .map_err(|_| "Export in progress".to_string())?;
        export_to(&directory, &source, &tenant).map_err(str::to_owned)
    })
    .await
    .map_err(|_| "Export worker failed".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    const SOURCE: &str = include_str!("../../../agents-builder-web/fixtures/hello.json");

    #[test]
    fn actual_compiler_is_inert_and_rejects_foreign_namespace() {
        let receipt = compile_local(SOURCE).unwrap();
        assert_eq!(receipt["effects_executed"], false);
        assert_eq!(receipt["namespace"], "local");
        assert_eq!(receipt["project_sha256"].as_str().unwrap().len(), 64);
        assert!(compile_local(&SOURCE.replace("\"local\"", "\"foreign\"")).is_err());
    }

    #[test]
    fn export_is_private_no_clobber_and_validated() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().canonicalize().unwrap();
        let first = export_to(&directory, SOURCE, "local").unwrap();
        let second = export_to(&directory, SOURCE, "local").unwrap();
        assert_ne!(first, second);
        let p = directory.join(first);
        let q = directory.join(second);
        assert_eq!(fs::read(&p).unwrap(), fs::read(&q).unwrap());
        assert_eq!(p.metadata().unwrap().permissions().mode() & 0o077, 0);
        assert!(export_to(&directory, SOURCE, "foreign").is_err());
        assert!(export_to(&directory, &SOURCE.replace("\"hello\"", "\"..\""), "local").is_err());
        let other = tempfile::tempdir().unwrap();
        let link = other.path().join("link");
        symlink(&directory, &link).unwrap();
        assert!(export_to(&link, SOURCE, "local").is_err());
    }
}
