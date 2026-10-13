#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use ocaf_runtime::executor::wasi::{
    manifest_signing_payload, validate_wasi_imports, PinnedWasmtime, ReceiptSigner, SandboxPolicy,
    WasmToolExecutor,
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
    let script = format!("#!/bin/sh\nwhile IFS= read -r _line; do :; done\n{body}\n");
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

/// Bounded, deterministic parser-mutation smoke test: malformed or partially
/// overwritten module bytes must never panic the admission validator.
#[test]
fn mutated_wasm_binaries_do_not_panic_import_validation() {
    let source = command_fixture(r#"{"safe":true}"#);
    for offset in 0..source.len() {
        for value in [0_u8, 0x7f, 0x80, 0xff] {
            let mut mutated = source.clone();
            mutated[offset] = value;
            let _ = validate_wasi_imports(&mutated);
        }
    }
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

/// The executable admitted for a signed tool cannot be writable by
/// another local user or have multiple hard-link aliases.
#[test]
fn pinned_engine_rejects_writable_or_multiply_linked_source() {
    let folder = tempfile::tempdir().expect("engine custody");
    let (path, digest) = fake_engine(folder.path(), "printf '{}'");
    let alias = folder.path().join("engine-alias");
    fs::hard_link(&path, &alias).expect("add second hard link");
    assert!(
        PinnedWasmtime::new(path.clone(), digest).is_err(),
        "engine with a second hard link must be rejected"
    );
    fs::remove_file(alias).expect("drop hard link");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o777))
        .expect("make group- and world-writable");
    assert!(
        PinnedWasmtime::new(path.clone(), digest).is_err(),
        "group/world-writable executable must be rejected"
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("restore trusted mode");
    PinnedWasmtime::new(path, digest).expect("sealed single-linked engine");
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

/// Error streams have the same bound as stdout and are not returned to
/// clients, even when malicious guests intentionally emit sensitive strings.
#[tokio::test]
async fn stderr_bomb_is_bounded_and_not_returned_to_callers() {
    let folder = tempfile::tempdir().expect("temporary stderr fixture");
    let (path, hash) = fake_engine(
        folder.path(),
        "printf 'untrusted-guest-secret' >&2; exec /usr/bin/yes >&2",
    );
    let exec = executor(
        path,
        hash,
        command_fixture("{}"),
        SandboxPolicy {
            max_output_bytes: 128,
            ..SandboxPolicy::default()
        },
    );
    let result = exec.execute(json!({}), &context()).await;
    match result {
        Err(ToolError::Validation(message)) => {
            assert!(!message.contains("untrusted-guest-secret"));
        }
        other => panic!("expected bounded stderr failure, got {other:?}"),
    }
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

/// Verify cancellation does not leave the Wasmtime child or module staging
/// directory behind. This checks a Linux subprocess boundary; it is not
/// evidence of cgroup/namespace/KVM isolation.
#[tokio::test]
async fn cancellation_reaps_guest_and_removes_private_workspace() {
    let folder = tempfile::tempdir().expect("temp");
    let pid_marker = folder.path().join("guest.pid");
    let cwd_marker = folder.path().join("guest.cwd");
    let script = format!(
        "echo \"$$\" > '{}'; pwd > '{}'; exec /bin/sleep 10",
        pid_marker.display(),
        cwd_marker.display()
    );
    let (path, hash) = fake_engine(folder.path(), &script);
    let executor = executor(
        path,
        hash,
        command_fixture("{}"),
        SandboxPolicy {
            timeout_ms: 10_000,
            ..SandboxPolicy::default()
        },
    );
    let mut ctx = context();
    ctx.deadline_ms = 10_000;
    let task = tokio::spawn(async move { executor.execute(json!({}), &ctx).await });

    let mut child_pid = None;
    for _ in 0..200 {
        if let (Ok(pid), Ok(cwd)) = (
            fs::read_to_string(&pid_marker),
            fs::read_to_string(&cwd_marker),
        ) {
            if let Ok(pid) = pid.trim().parse::<u32>() {
                if !cwd.trim().is_empty() {
                    child_pid = Some((pid, cwd.trim().to_owned()));
                    break;
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let (pid, guest_workspace) = child_pid.expect("guest started and recorded its workspace");
    assert!(Path::new(&guest_workspace).is_dir());

    task.abort();
    let _ = task.await;

    // The Tokio child-process reaper may complete asynchronously.
    for _ in 0..200 {
        let guest_gone = !Path::new(&format!("/proc/{pid}")).exists();
        let workspace_gone = !Path::new(&guest_workspace).exists();
        if guest_gone && workspace_gone {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("cancelled WASI subprocess or temporary module workspace was not cleaned up");
}

/// A cancelled guest must not leave a background process in its process
/// group. Unlike a regular Wasmtime module, this shell fixture intentionally
/// creates a descendant to exercise the subprocess supervisor boundary.
#[tokio::test]
async fn cancelling_guest_kills_background_descendants() {
    let folder = tempfile::tempdir().expect("temporary fixture");
    let descendant_pid_file = folder.path().join("descendant.pid");
    let ready_file = folder.path().join("started");
    let shell = format!(
        "/bin/sleep 10 & echo \"$!\" > '{}'; echo ready > '{}'; wait",
        descendant_pid_file.display(),
        ready_file.display()
    );
    let (path, hash) = fake_engine(folder.path(), &shell);
    let exec = executor(
        path,
        hash,
        command_fixture("{}"),
        SandboxPolicy {
            timeout_ms: 10_000,
            ..SandboxPolicy::default()
        },
    );
    let mut ctx = context();
    ctx.deadline_ms = 10_000;
    let task = tokio::spawn(async move { exec.execute(json!({}), &ctx).await });
    let mut pid = None;
    for _ in 0..150 {
        if ready_file.is_file() {
            if let Ok(value) = fs::read_to_string(&descendant_pid_file) {
                pid = value.trim().parse::<u32>().ok();
                if pid.is_some() {
                    break;
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let pid = pid.expect("guest started background child");

    fn running(pid: u32) -> bool {
        let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) else {
            return false;
        };
        let Some((_, suffix)) = stat.rsplit_once(") ") else {
            return false;
        };
        !matches!(suffix.chars().next(), Some('Z' | 'X'))
    }

    assert!(
        running(pid),
        "background child was not alive before cancellation"
    );
    task.abort();
    let _ = task.await;

    let mut stopped = false;
    for _ in 0..120 {
        if !running(pid) {
            stopped = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    if !stopped {
        // The RED test must clean up its fixture even on an expected failure.
        unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
    }
    assert!(
        stopped,
        "WASI cancellation left a live background descendant"
    );
}

/// Linux subprocess defense-in-depth: prevent gaining privileges via exec
/// and suppress core files containing module input or runtime secrets.
/// A coordinator killed with SIGKILL cannot run Rust Drop handlers.
/// The guest leader must inherit a kernel-level parent-death signal.
#[tokio::test]
async fn wasi_guest_leader_receives_sigkill_on_parent_death() {
    let folder = tempfile::tempdir().expect("temp");
    // PR_SET_PDEATHSIG is intentionally not inherited by grandchildren.
    // Use a direct python shebang to inspect the *same guest leader PID*
    // after exec, not a shell-spawned grandchild which would always see 0.
    let script = b"#!/usr/bin/python3\\nimport ctypes, json\\nlibc = ctypes.CDLL(None)\\nout = ctypes.c_int()\\nif libc.prctl(2, ctypes.byref(out), 0, 0, 0) != 0: raise SystemExit(81)\\nprint(json.dumps({'parent_death_signal': out.value}))\\n";
    let path = folder.path().join("guest-leader");
    fs::write(&path, script).expect("write guest leader");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("chmod guest leader");
    let actual = executor(path, digest(script), command_fixture("{}"), SandboxPolicy::default())
        .execute(json!({}), &context())
        .await
        .expect("guest leader parent death policy");
    assert_eq!(actual, json!({"parent_death_signal": libc::SIGKILL}));

}

#[tokio::test]
async fn wasi_guest_inherits_no_new_privileges_and_core_limit() {
    let folder = tempfile::tempdir().expect("temp");
    let status_file = folder.path().join("child-hardening-status");
    let probe = format!(
        r#"
nnp=0
while read -r key value rest; do
    if [ "$key" = "NoNewPrivs:" ] && [ "$value" = "1" ]; then nnp=1; fi
done < /proc/self/status
echo "$nnp:$(ulimit -c)" > '{}'
echo '"healthy"'
"#,
        status_file.display()
    );
    let (path, hash) = fake_engine(folder.path(), &probe);
    let result = executor(path, hash, command_fixture("{}"), SandboxPolicy::default())
        .execute(json!({}), &context())
        .await;
    assert_eq!(
        result.expect("Linux child hardening policy"),
        json!("healthy")
    );
    let observed = fs::read_to_string(status_file).expect("recorded Linux child limits");
    assert_eq!(
        observed.trim(),
        "1:0",
        "child must have no_new_privs and no core"
    );
}

#[tokio::test]
async fn wasi_pool_exhaustion_rejects_without_waiting_for_unsafe_fallback() {
    let folder = tempfile::tempdir().expect("tmp");
    let ready = folder.path().join("guest-ready");
    let shell = format!("echo ready > '{}'; exec /bin/sleep 5", ready.display());
    let (path, hash) = fake_engine(folder.path(), &shell);
    let policy = SandboxPolicy {
        max_concurrent: 1,
        timeout_ms: 1000,
        ..SandboxPolicy::default()
    };
    let exec = executor(path, hash, command_fixture("{}"), policy);
    let first = {
        let cloned = exec.clone();
        tokio::spawn(async move { cloned.execute(json!({}), &context()).await })
    };
    let mut observed = false;
    for _ in 0..100 {
        if ready.is_file() {
            observed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(observed, "first guest never started");
    let second = exec.execute(json!({}), &context()).await;
    assert!(matches!(second, Err(ToolError::Validation(_))));
    first.abort();
    let _ = first.await;
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

    // A bounded smoke cycle with the actual pinned engine catches reuse,
    // stale subprocess state and repeated private-workspace cleanup defects.
    for _ in 0..8 {
        let again = exec.execute(json!({}), &context()).await.expect("repeat");
        assert_eq!(result, again);
    }

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

#[tokio::test]
async fn publisher_signature_binds_module_tool_policy_and_principal() {
    use openssl::pkey::PKey;
    use openssl::sign::Signer;
    use std::time::{SystemTime, UNIX_EPOCH};

    let key = PKey::generate_ed25519().expect("ephemeral Ed25519 key");
    let public = key.public_key_to_pem().expect("PEM public key");
    let invalid_key = PKey::generate_ed25519().expect("different signer");
    let other_public = invalid_key.public_key_to_pem().expect("other PEM");

    let module = command_fixture(r#"{"signed":true}"#);
    let module_hex = format!("{:x}", Sha256::digest(&module));
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("monotonic epoch")
        .as_secs();
    let tool = ToolManifest {
        name: "signed_wasi".into(),
        description: "signed fixture".into(),
        input_schema: json!({"type":"object"}),
        output_schema: json!({"type":"object"}),
        cost_sats: 0,
        permissions: Vec::new(),
        timeout_ms: 1000,
        enabled: true,
    };
    let document = json!({
        "schema_version": 1,
        "tenant_id": "tenant-a",
        "agent_id": "agent-1",
        "tool": tool,
        "module_sha256_hex": module_hex,
        "expires_unix_secs": now + 3600,
        "policy": SandboxPolicy::default()
    });
    let json = serde_json::to_vec(&document).expect("manifest JSON");
    let payload = manifest_signing_payload(&json).expect("domain-separated payload");
    let mut signer = Signer::new_without_digest(&key).expect("Ed25519 signer");
    let signature = signer.sign_oneshot_to_vec(&payload).expect("sign payload");

    let folder = tempfile::tempdir().expect("tmp");
    let (engine, engine_hash) = fake_engine(folder.path(), "printf '{\"signed\":true}\\n'");
    let approved = PinnedWasmtime::new(engine.clone(), engine_hash).expect("pinned");
    let exec = WasmToolExecutor::from_signed_manifest(
        approved.clone(),
        module.clone(),
        &json,
        &signature,
        &public,
    )
    .expect("valid publisher signature");
    assert_eq!(
        exec.signed_tool_manifest().expect("manifest").name,
        "signed_wasi"
    );

    let unsigned = executor(
        engine.clone(),
        engine_hash,
        module.clone(),
        SandboxPolicy::default(),
    );
    assert!(unsigned.signed_tool_manifest().is_none());
    assert!(WasmToolExecutor::from_signed_manifest(
        approved.clone(),
        module.clone(),
        &json,
        &signature,
        &other_public
    )
    .is_err());
    let mut modified = json.clone();
    modified.push(b' ');
    assert!(WasmToolExecutor::from_signed_manifest(
        approved.clone(),
        module.clone(),
        &modified,
        &signature,
        &public
    )
    .is_err());
    assert!(WasmToolExecutor::from_signed_manifest(
        approved.clone(),
        command_fixture("{}"),
        &json,
        &signature,
        &public
    )
    .is_err());

    let mut context = context();
    assert!(exec.execute(json!({}), &context).await.is_err());
    context.agent_id = "tenant-b:agent-1".into();
    assert!(exec.execute(json!({}), &context).await.is_err());
    context.agent_id = "tenant-a:agent-1".into();
    assert_eq!(
        exec.execute(json!({}), &context)
            .await
            .expect("authorized request"),
        json!({"signed":true})
    );

    let signing_key = ReceiptSigner::new([7u8; 32]).expect("operator receipt key");
    let (verified_output, signed_receipt) = exec
        .execute_with_receipt(json!({"request":1}), &context, &signing_key)
        .await
        .expect("execution attestation");
    assert_eq!(verified_output, json!({"signed":true}));
    signing_key.verify(&signed_receipt).expect("valid receipt");
    let mut tampered = signed_receipt.clone();
    tampered.receipt.agent_id = "attacker".into();
    assert!(signing_key.verify(&tampered).is_err());
    assert!(ReceiptSigner::new([0u8; 32]).is_err());

    let signed_runtime = OcafRuntime::new(0)
        .with_tenant_namespace("tenant-a")
        .expect("configured tenant");
    signed_runtime
        .register_signed_wasi(exec.clone())
        .await
        .expect("signed-only registration");
    assert!(signed_runtime.register_signed_wasi(unsigned).await.is_err());
    let call = ToolCallRequest {
        call_id: "signed-1".into(),
        tool: "signed_wasi".into(),
        args: json!({}),
        auth: None,
    };
    assert!(signed_runtime.execute(call.clone()).await.is_err());
    assert!(signed_runtime
        .execute_authenticated_peer(call.clone(), "agent-2")
        .await
        .is_err());
    assert_eq!(
        signed_runtime
            .execute_authenticated_peer(call.clone(), "agent-1")
            .await
            .expect("verified scoped RPC caller")
            .output,
        json!({"signed":true})
    );
    let other_tenant = OcafRuntime::new(0)
        .with_tenant_namespace("tenant-b")
        .expect("other tenant");
    other_tenant
        .register_signed_wasi(exec.clone())
        .await
        .expect("register other tenant");
    assert!(other_tenant
        .execute_authenticated_peer(call, "agent-1")
        .await
        .is_err());

    let expired = json!({
        "schema_version": 1,
        "tenant_id": "tenant-a",
        "agent_id": "agent-1",
        "tool": tool,
        "module_sha256_hex": module_hex,
        "expires_unix_secs": now - 1,
        "policy": SandboxPolicy::default()
    });
    let expired_json = serde_json::to_vec(&expired).expect("expired manifest");
    let mut expired_signer = Signer::new_without_digest(&key).expect("signer");
    let expired_sig = expired_signer
        .sign_oneshot_to_vec(&manifest_signing_payload(&expired_json).expect("payload"))
        .expect("sign");
    assert!(WasmToolExecutor::from_signed_manifest(
        approved,
        module,
        &expired_json,
        &expired_sig,
        &public
    )
    .is_err());
}
