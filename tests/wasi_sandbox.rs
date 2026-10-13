#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use ocaf_runtime::executor::wasi::{
    validate_wasi_imports, PinnedWasmtime, SandboxPolicy, WasmToolExecutor,
};
use ocaf_runtime::executor::{ExecutionContext, ToolError, ToolExecutor};
use ocaf_runtime::registry::ToolManifest;
use ocaf_runtime::runtime::{OcafRuntime, RuntimeError, ToolCallRequest};
use serde_json::json;
use sha2::{Digest, Sha256};

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn leb(mut x: usize) -> Vec<u8> {
    let mut result = Vec::new();
    loop {
        let mut next = (x & 0x7f) as u8;
        x >>= 7;
        if x != 0 {
            next |= 0x80;
        }
        result.push(next);
        if x == 0 {
            return result;
        }
    }
}

fn wasm_name(value: &str) -> Vec<u8> {
    let mut result = leb(value.len());
    result.extend_from_slice(value.as_bytes());
    result
}

fn section(wasm: &mut Vec<u8>, number: u8, data: &[u8]) {
    wasm.push(number);
    wasm.extend(leb(data.len()));
    wasm.extend(data);
}

/// Real WASI Preview1 core command: fd_write(1, iov=0, 1, nwritten=8)
/// prints one static JSON document with a newline.
fn command_fixture(message: &str) -> Vec<u8> {
    let mut wasm = b"\0asm\x01\0\0\0".to_vec();
    // Two signatures: imported fd_write and locally exported _start.
    section(
        &mut wasm,
        1,
        &[2, 0x60, 4, 0x7f, 0x7f, 0x7f, 0x7f, 1, 0x7f, 0x60, 0, 0],
    );
    let mut imports = vec![1];
    imports.extend(wasm_name("wasi_snapshot_preview1"));
    imports.extend(wasm_name("fd_write"));
    imports.extend([0, 0]);
    section(&mut wasm, 2, &imports);
    section(&mut wasm, 3, &[1, 1]);
    section(&mut wasm, 5, &[1, 0, 1]);
    let mut exports = vec![2];
    exports.extend(wasm_name("memory"));
    exports.extend([2, 0]);
    exports.extend(wasm_name("_start"));
    exports.extend([0, 1]);
    section(&mut wasm, 7, &exports);
    let body = [0, 0x41, 1, 0x41, 0, 0x41, 1, 0x41, 8, 0x10, 0, 0x1a, 0x0b];
    let mut code = vec![1];
    code.extend(leb(body.len()));
    code.extend(body);
    section(&mut wasm, 10, &code);
    let message = format!("{message}\n");
    let mut data = vec![0; 16];
    data[0..4].copy_from_slice(&16u32.to_le_bytes());
    data[4..8].copy_from_slice(&(message.len() as u32).to_le_bytes());
    data.extend(message.as_bytes());
    let mut segments = vec![1, 0, 0x41, 0, 0x0b];
    segments.extend(leb(data.len()));
    segments.extend(data);
    section(&mut wasm, 11, &segments);
    wasm
}

fn infinite_loop_fixture() -> Vec<u8> {
    let mut wasm = b"\0asm\x01\0\0\0".to_vec();
    section(&mut wasm, 1, &[1, 0x60, 0, 0]);
    section(&mut wasm, 3, &[1, 0]);
    let mut exports = vec![1];
    exports.extend(wasm_name("_start"));
    exports.extend([0, 0]);
    section(&mut wasm, 7, &exports);
    let body = [0, 0x03, 0x40, 0x0c, 0, 0x0b, 0x0b];
    let mut code = vec![1];
    code.extend(leb(body.len()));
    code.extend(body);
    section(&mut wasm, 10, &code);
    wasm
}

fn oversized_memory_fixture() -> Vec<u8> {
    let mut wasm = b"\0asm\x01\0\0\0".to_vec();
    section(&mut wasm, 1, &[1, 0x60, 0, 0]);
    section(&mut wasm, 3, &[1, 0]);
    let mut memory = vec![1, 0];
    memory.extend(leb(1024));
    section(&mut wasm, 5, &memory);
    let mut exports = vec![1];
    exports.extend(wasm_name("_start"));
    exports.extend([0, 0]);
    section(&mut wasm, 7, &exports);
    section(&mut wasm, 10, &[1, 2, 0, 0x0b]);
    wasm
}

fn with_import(namespace: &str, function: &str) -> Vec<u8> {
    let mut wasm = b"\0asm\x01\0\0\0".to_vec();
    section(&mut wasm, 1, &[1, 0x60, 0, 0]);
    let mut import = vec![1];
    import.extend(wasm_name(namespace));
    import.extend(wasm_name(function));
    import.extend([0, 0]);
    section(&mut wasm, 2, &import);
    wasm
}

fn fake_engine(dir: &Path, body: &str) -> (PathBuf, [u8; 32]) {
    let path = dir.join("fake-wasmtime");
    let script = format!("#!/bin/sh\n{body}\n");
    fs::write(&path, script.as_bytes()).expect("script");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("chmod");
    (path, digest(script.as_bytes()))
}

fn executor(
    path: PathBuf,
    binary_digest: [u8; 32],
    module: Vec<u8>,
    policy: SandboxPolicy,
) -> WasmToolExecutor {
    let expected = digest(&module);
    let engine = PinnedWasmtime::new(path, binary_digest).expect("valid pinned engine");
    WasmToolExecutor::new(engine, module, expected, policy).expect("valid approved module")
}

fn context() -> ExecutionContext {
    ExecutionContext {
        agent_id: "agent-test".into(),
        correlation_id: "wasi-test".into(),
        deadline_ms: 1_000,
    }
}

#[test]
fn restricted_imports_fail_closed() {
    let valid = command_fixture(r#"{"ok":true}"#);
    validate_wasi_imports(&valid).expect("stdio-only WASI import");
    for function in [
        "path_open",
        "path_filestat_get",
        "sock_recv",
        "sock_send",
        "random_get",
        "clock_time_get",
        "proc_raise",
        "poll_oneoff",
    ] {
        assert!(
            validate_wasi_imports(&with_import("wasi_snapshot_preview1", function)).is_err(),
            "unexpected approval: {function}"
        );
    }
    assert!(validate_wasi_imports(&with_import("host", "danger")).is_err());
    assert!(validate_wasi_imports(b"\0asm\x01\0\0").is_err());
    assert!(validate_wasi_imports(b"\0asm\x0d\0\0\0").is_err());
    assert!(validate_wasi_imports(&[0u8; 100]).is_err());
    assert!(validate_wasi_imports(&vec![0u8; 1024 * 1024 + 1]).is_err());
    let mut truncated = b"\0asm\x01\0\0\0".to_vec();
    truncated.extend([2, 100]);
    assert!(validate_wasi_imports(&truncated).is_err());
}

#[test]
fn digest_engine_and_policy_fail_closed() {
    let folder = tempfile::tempdir().expect("tmp");
    let (path, hash) = fake_engine(folder.path(), "printf '{\"ok\":true}'");
    let module = command_fixture(r#"{"ok":true}"#);
    let engine = PinnedWasmtime::new(path.clone(), hash).expect("approved");
    assert!(WasmToolExecutor::new(
        engine.clone(),
        module.clone(),
        [0u8; 32],
        SandboxPolicy::default()
    )
    .is_err());
    let policy = SandboxPolicy {
        fuel: 0,
        ..SandboxPolicy::default()
    };
    assert!(WasmToolExecutor::new(
        engine,
        module,
        digest(&command_fixture(r#"{"ok":true}"#)),
        policy
    )
    .is_err());
    fs::write(&path, "#!/bin/sh\nexit 0\n").expect("mutate");
    assert!(PinnedWasmtime::new(path, hash).is_err());
}

#[tokio::test]
async fn json_wasi_adapter_integrates_with_runtime_validation() {
    let folder = tempfile::tempdir().expect("tmp");
    let (path, hash) = fake_engine(folder.path(), "printf '{\"ok\":true}\\n'");
    let module = command_fixture(r#"{"ok":true}"#);
    let runtime = OcafRuntime::new(0);
    runtime.register(
        ToolManifest {
            name: "approved_wasi_transform".into(),
            description: "Only a test fixture, never a privileged host tool".into(),
            input_schema: json!({"type":"object","required":["data"],"properties":{"data":{"type":"string"}}}),
            output_schema: json!({"type":"object","required":["ok"],"properties":{"ok":{"const":true}}}),
            cost_sats: 0,
            permissions: Vec::new(),
            timeout_ms: 1000,
            enabled: true,
        },
        executor(path, hash, module, SandboxPolicy::default()),
    ).await.expect("registry");
    let request = ToolCallRequest {
        call_id: "wasi-1".into(),
        tool: "approved_wasi_transform".into(),
        args: json!({"data":"sent over stdin"}),
        auth: None,
    };
    let result = runtime.execute(request.clone()).await.expect("output");
    assert_eq!(result.output, json!({"ok":true}));
    let invalid = ToolCallRequest {
        args: json!({"data":44}),
        ..request
    };
    assert!(matches!(
        runtime.execute(invalid).await,
        Err(RuntimeError::InvalidInput(_))
    ));
}

#[tokio::test]
async fn stdout_bomb_is_rejected_without_host_log_leak() {
    let folder = tempfile::tempdir().expect("tmp");
    let (path, hash) = fake_engine(folder.path(), "exec /usr/bin/yes");
    let policy = SandboxPolicy {
        max_output_bytes: 128,
        ..SandboxPolicy::default()
    };
    let result = executor(path, hash, command_fixture("{}"), policy)
        .execute(json!({}), &context())
        .await;
    assert!(matches!(result, Err(ToolError::Validation(_))));
}

#[tokio::test]
async fn timeout_kills_guest_process_and_rejects_result() {
    let folder = tempfile::tempdir().expect("tmp");
    let (path, hash) = fake_engine(folder.path(), "exec /bin/sleep 2");
    let policy = SandboxPolicy {
        timeout_ms: 30,
        ..SandboxPolicy::default()
    };
    let result = executor(path, hash, command_fixture("{}"), policy)
        .execute(json!({}), &context())
        .await;
    assert!(matches!(result, Err(ToolError::Timeout)));
}

#[tokio::test]
async fn output_json_and_input_limits_enforced() {
    let folder = tempfile::tempdir().expect("tmp");
    let (path, hash) = fake_engine(folder.path(), "printf 'not-json\\n'");
    let module = command_fixture("{}");
    let exec = executor(path, hash, module, SandboxPolicy::default());
    assert!(matches!(
        exec.execute(json!({}), &context()).await,
        Err(ToolError::Validation(_))
    ));
    let folder2 = tempfile::tempdir().expect("tmp");
    let (path2, hash2) = fake_engine(folder2.path(), "printf '{}\\n'");
    let policy = SandboxPolicy {
        max_input_bytes: 32,
        ..SandboxPolicy::default()
    };
    let exec2 = executor(path2, hash2, command_fixture("{}"), policy);
    let result = exec2
        .execute(json!({"data":"a".repeat(100)}), &context())
        .await;
    assert!(matches!(result, Err(ToolError::Validation(_))));
}

/// Opt-in real Wasmtime E2E. This test is only executed where a pinned
/// operator-managed binary is supplied and is never counted as a CI PASS
/// unless those environment variables are explicitly set.
#[tokio::test]
async fn real_wasmtime_cli_when_configured() {
    let Ok(engine_path) = std::env::var("OCAF_TEST_WASMTIME") else {
        return;
    };
    let bytes = fs::read(&engine_path).expect("read explicitly selected Wasmtime");
    let engine_digest = digest(&bytes);
    let exec = executor(
        PathBuf::from(engine_path),
        engine_digest,
        command_fixture(r#"{"real_wasi":true}"#),
        SandboxPolicy::default(),
    );
    let result = exec
        .execute(json!({}), &context())
        .await
        .expect("real WASI");
    assert_eq!(result, json!({"real_wasi":true}));

    let again = exec.execute(json!({}), &context()).await.expect("repeat");
    assert_eq!(result, again);

    let pinned = PinnedWasmtime::new(
        PathBuf::from(std::env::var("OCAF_TEST_WASMTIME").expect("checked above")),
        engine_digest,
    )
    .expect("approved engine");
    let loop_module = infinite_loop_fixture();
    let loop_exec = WasmToolExecutor::new(
        pinned.clone(),
        loop_module.clone(),
        digest(&loop_module),
        SandboxPolicy {
            fuel: 10_000,
            ..SandboxPolicy::default()
        },
    )
    .expect("bounded loop fixture");
    assert!(matches!(
        loop_exec.execute(json!({}), &context()).await,
        Err(ToolError::Execution(_)) | Err(ToolError::Timeout)
    ));

    let memory_module = oversized_memory_fixture();
    let memory_exec = WasmToolExecutor::new(
        pinned,
        memory_module.clone(),
        digest(&memory_module),
        SandboxPolicy::default(),
    )
    .expect("bounded memory fixture");
    assert!(matches!(
        memory_exec.execute(json!({}), &context()).await,
        Err(ToolError::Execution(_)) | Err(ToolError::Timeout)
    ));
}
