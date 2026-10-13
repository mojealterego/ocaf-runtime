//! Restricted WASI Preview 1 command execution through a separately installed
//! and SHA-256 pinned Wasmtime CLI. This is an opt-in execution adapter, not
//! a host or microVM security boundary.
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::Semaphore;
use uuid::Uuid;

use super::{ExecutionContext, ToolError, ToolExecutor};

const MAX_MODULE_BYTES: usize = 1024 * 1024;
const MAX_IO_BYTES: usize = 64 * 1024;
const MAX_MEMORY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EXECUTION_MS: u64 = 10_000;
const WASM_MAGIC_VERSION: &[u8; 8] = b"\0asm\x01\0\0\0";
const ALLOWED_WASI_IMPORTS: &[&str] = &[
    "fd_read",
    "fd_write",
    "fd_fdstat_get",
    "proc_exit",
];

fn rejected(reason: &str) -> ToolError {
    ToolError::Validation(reason.to_owned())
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn read_uleb(bytes: &[u8], offset: &mut usize) -> Result<usize, ToolError> {
    let mut value = 0usize;
    for shift in (0..35).step_by(7) {
        let byte = *bytes
            .get(*offset)
            .ok_or_else(|| rejected("truncated wasm unsigned integer"))?;
        *offset += 1;
        if shift >= usize::BITS as usize || ((byte & 0x7f) as usize) > (usize::MAX >> shift) {
            return Err(rejected("overflowing wasm unsigned integer"));
        }
        value |= ((byte & 0x7f) as usize) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(rejected("overlong wasm unsigned integer"))
}

fn read_name<'a>(bytes: &'a [u8], cursor: &mut usize) -> Result<&'a str, ToolError> {
    let len = read_uleb(bytes, cursor)?;
    let end = cursor
        .checked_add(len)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| rejected("truncated wasm name"))?;
    let name = std::str::from_utf8(&bytes[*cursor..end])
        .map_err(|_| rejected("invalid wasm import name"))?;
    *cursor = end;
    Ok(name)
}

/// Conservatively accept only basic WASI Preview 1 stdin/stdout calls.
/// Every other import, including clocks, randomness, filesystem paths,
/// sockets, WASIp2 component imports and host extensions, is denied.
/// Wasmtime still performs the authoritative full Wasm binary validation.
pub fn validate_wasi_imports(bytes: &[u8]) -> Result<(), ToolError> {
    if bytes.len() > MAX_MODULE_BYTES || !bytes.starts_with(WASM_MAGIC_VERSION) {
        return Err(rejected("module size, wasm magic, or core version rejected"));
    }
    let mut cursor = WASM_MAGIC_VERSION.len();
    let mut seen_import_section = false;
    while cursor < bytes.len() {
        let section_id = bytes[cursor];
        cursor += 1;
        let len = read_uleb(bytes, &mut cursor)?;
        let end = cursor
            .checked_add(len)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| rejected("invalid wasm section length"))?;
        if section_id == 2 {
            if seen_import_section {
                return Err(rejected("duplicate wasm import section"));
            }
            seen_import_section = true;
            let mut pos = cursor;
            let count = read_uleb(&bytes[..end], &mut pos)?;
            if count > 128 {
                return Err(rejected("too many wasm imports"));
            }
            for _ in 0..count {
                let namespace = read_name(&bytes[..end], &mut pos)?;
                let name = read_name(&bytes[..end], &mut pos)?;
                let kind = *bytes.get(pos).ok_or_else(|| rejected("truncated wasm import"))?;
                pos += 1;
                if kind != 0
                    || namespace != "wasi_snapshot_preview1"
                    || !ALLOWED_WASI_IMPORTS.contains(&name)
                {
                    return Err(rejected("disallowed wasm host import"));
                }
                read_uleb(&bytes[..end], &mut pos)?;
            }
            if pos != end {
                return Err(rejected("trailing data in wasm import section"));
            }
        }
        cursor = end;
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct SandboxPolicy {
    pub fuel: u64,
    pub max_linear_memory_bytes: u64,
    pub max_wasm_stack_bytes: u64,
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub timeout_ms: u64,
    pub max_concurrent: usize,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            fuel: 2_000_000,
            max_linear_memory_bytes: 32 * 1024 * 1024,
            max_wasm_stack_bytes: 1024 * 1024,
            max_input_bytes: 16 * 1024,
            max_output_bytes: 16 * 1024,
            timeout_ms: 2_000,
            max_concurrent: 4,
        }
    }
}

impl SandboxPolicy {
    fn validate(&self) -> Result<(), ToolError> {
        if self.fuel == 0
            || self.max_linear_memory_bytes == 0
            || self.max_linear_memory_bytes > MAX_MEMORY_BYTES
            || self.max_wasm_stack_bytes == 0
            || self.max_wasm_stack_bytes > 2 * 1024 * 1024
            || self.max_input_bytes == 0
            || self.max_input_bytes > MAX_IO_BYTES
            || self.max_output_bytes == 0
            || self.max_output_bytes > MAX_IO_BYTES
            || self.timeout_ms == 0
            || self.timeout_ms > MAX_EXECUTION_MS
            || self.max_concurrent == 0
            || self.max_concurrent > 32
        {
            return Err(rejected("invalid WASI sandbox policy limits"));
        }
        Ok(())
    }
}

fn hash_file(path: &Path) -> Result<[u8; 32], ToolError> {
    let mut file = File::open(path)
        .map_err(|_| rejected("pinned WASI engine is unavailable"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 16_384];
    loop {
        let size = file
            .read(&mut buf)
            .map_err(|_| rejected("cannot verify WASI engine"))?;
        if size == 0 {
            break;
        }
        hasher.update(&buf[..size]);
    }
    Ok(hasher.finalize().into())
}

/// Operator-provided, absolute, executable engine path and verified digest.
/// The caller must protect the executable and all parent directories from
/// untrusted mutation. No PATH lookup and no engine downloaded at runtime.
#[derive(Clone)]
pub struct PinnedWasmtime {
    executable: PathBuf,
    sha256: [u8; 32],
}

impl PinnedWasmtime {
    pub fn new(executable: PathBuf, expected_sha256: [u8; 32]) -> Result<Self, ToolError> {
        let engine = Self {
            executable,
            sha256: expected_sha256,
        };
        engine.verify()?;
        Ok(engine)
    }

    fn verify(&self) -> Result<(), ToolError> {
        if !self.executable.is_absolute() {
            return Err(rejected("WASI engine path must be absolute"));
        }
        let meta = fs::symlink_metadata(&self.executable)
            .map_err(|_| rejected("WASI engine metadata unavailable"))?;
        if !meta.is_file() || meta.file_type().is_symlink() || meta.permissions().mode() & 0o111 == 0 {
            return Err(rejected("WASI engine must be a regular executable, not a symlink"));
        }
        if hash_file(&self.executable)? != self.sha256 {
            return Err(rejected("WASI engine digest mismatch"));
        }
        Ok(())
    }
}

struct EphemeralModule {
    directory: PathBuf,
    module_path: PathBuf,
}

impl EphemeralModule {
    fn new(module: &[u8]) -> Result<Self, ToolError> {
        let directory = PathBuf::from("/tmp").join(format!("ocaf-wasi-{}", Uuid::new_v4()));
        DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .map_err(|_| rejected("cannot create private WASI workspace"))?;
        let module_path = directory.join("approved.wasm");
        let owner = Self { directory, module_path };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&owner.module_path)
            .map_err(|_| rejected("cannot stage WASI module"))?;
        file.write_all(module)
            .map_err(|_| rejected("cannot write staged WASI module"))?;
        Ok(owner)
    }
}

impl Drop for EphemeralModule {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.module_path);
        let _ = fs::remove_dir(&self.directory);
    }
}

/// WASI command contract: stdin contains one JSON document followed by LF;
/// stdout must contain one JSON document (whitespace permitted).
/// Authoritative ACL, schema validation, cost and replay/fencing checks remain
/// outside this adapter in OcafRuntime / the RPC coordinator.
#[derive(Clone)]
pub struct WasmToolExecutor {
    policy: SandboxPolicy,
    engine: PinnedWasmtime,
    approved_module: Arc<[u8]>,
    approved_sha256: [u8; 32],
    permits: Arc<Semaphore>,
}

impl WasmToolExecutor {
    pub fn new(
        engine: PinnedWasmtime,
        module: Vec<u8>,
        expected_module_sha256: [u8; 32],
        policy: SandboxPolicy,
    ) -> Result<Self, ToolError> {
        policy.validate()?;
        if sha256(&module) != expected_module_sha256 {
            return Err(rejected("WASI module digest mismatch"));
        }
        validate_wasi_imports(&module)?;
        Ok(Self {
            permits: Arc::new(Semaphore::new(policy.max_concurrent)),
            approved_module: Arc::from(module),
            approved_sha256: expected_module_sha256,
            engine,
            policy,
        })
    }

    async fn run(&self, args: Value, ctx: &ExecutionContext) -> Result<Value, ToolError> {
        let _permit = self.permits.acquire().await
            .map_err(|_| rejected("WASI sandbox admission closed"))?;
        if ctx.deadline_ms == 0 {
            return Err(rejected("deadline is required"));
        }
        let mut payload = serde_json::to_vec(&args)
            .map_err(|_| rejected("WASI input is not serializable"))?;
        if payload.len() >= self.policy.max_input_bytes {
            return Err(rejected("WASI input exceeds policy limit"));
        }
        payload.push(b'\n');

        // Reject replacement of approved bytes or operator-managed engine.
        if sha256(&self.approved_module) != self.approved_sha256 {
            return Err(rejected("approved WASI module changed"));
        }
        self.engine.verify()?;
        let staged = EphemeralModule::new(&self.approved_module)?;

        let mut command = Command::new(&self.engine.executable);
        command
            .arg("run")
            .arg("-W")
            .arg(format!(
                "fuel={},max-memory-size={},max-wasm-stack={},max-instances=1,max-memories=1,max-tables=1,nan-canonicalization=y",
                self.policy.fuel,
                self.policy.max_linear_memory_bytes,
                self.policy.max_wasm_stack_bytes
            ))
            .arg(&staged.module_path)
            .env_clear()
            .current_dir(&staged.directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = command.spawn()
            .map_err(|_| ToolError::Execution("cannot start pinned WASI runtime".into()))?;
        let mut stdin = child.stdin.take()
            .ok_or_else(|| ToolError::Execution("WASI stdin unavailable".into()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| ToolError::Execution("WASI stdout unavailable".into()))?;
        let stderr = child.stderr.take()
            .ok_or_else(|| ToolError::Execution("WASI stderr unavailable".into()))?;

        let limit = self.policy.max_output_bytes;
        let timeout = Duration::from_millis(self.policy.timeout_ms.min(ctx.deadline_ms));
        let completed = tokio::time::timeout(timeout, async {
            let send = async {
                stdin.write_all(&payload).await
                    .map_err(|_| ToolError::Execution("WASI stdin write failed".into()))?;
                stdin.shutdown().await
                    .map_err(|_| ToolError::Execution("WASI stdin shutdown failed".into()))?;
                Ok::<(), ToolError>(())
            };
            let output = read_bounded(stdout, limit);
            let error_output = read_bounded(stderr, limit);
            let exit = async {
                child.wait().await
                    .map_err(|_| ToolError::Execution("WASI process wait failed".into()))
            };
            tokio::try_join!(send, output, error_output, exit)
        }).await.map_err(|_| ToolError::Timeout)??;

        let ((), output, _stderr, status) = completed;
        if !status.success() {
            return Err(ToolError::Execution("WASI guest failed or trapped".into()));
        }
        serde_json::from_slice(&output)
            .map_err(|_| ToolError::Validation("WASI guest returned invalid JSON".into()))
    }
}

#[async_trait::async_trait]
impl ToolExecutor for WasmToolExecutor {
    async fn execute(&self, args: Value, ctx: &ExecutionContext) -> Result<Value, ToolError> {
        self.run(args, ctx).await
    }
}

async fn read_bounded<R>(mut reader: R, max_bytes: usize) -> Result<Vec<u8>, ToolError>
where
    R: AsyncRead + Unpin,
{
    let mut result = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let size = reader.read(&mut buffer).await
            .map_err(|_| ToolError::Execution("WASI output read failed".into()))?;
        if size == 0 {
            return Ok(result);
        }
        if size > max_bytes.saturating_sub(result.len()) {
            return Err(rejected("WASI output exceeded size limit"));
        }
        result.extend_from_slice(&buffer[..size]);
    }
}
