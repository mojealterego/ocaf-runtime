//! Restricted WASI Preview 1 command execution through a separately installed
//! and SHA-256 pinned Wasmtime CLI. This is an opt-in execution adapter, not
//! a host or microVM security boundary.
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::Semaphore;
use uuid::Uuid;

use super::{ExecutionContext, ToolError, ToolExecutor};

#[path = "wasi_manifest.rs"]
mod signed_manifest;
pub use signed_manifest::{manifest_signing_payload, verify_signed_manifest, SignedWasiManifest};
#[path = "wasi_receipt.rs"]
mod receipts;
pub use receipts::{ExecutionReceipt, ReceiptSigner, SignedExecutionReceipt};

const MAX_MODULE_BYTES: usize = 1024 * 1024;
const MAX_IO_BYTES: usize = 64 * 1024;
const MAX_MEMORY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EXECUTION_MS: u64 = 10_000;
const MAX_FUEL: u64 = 50_000_000;
const MAX_ENGINE_BYTES: u64 = 256 * 1024 * 1024;
const WASM_MAGIC_VERSION: &[u8; 8] = b"\0asm\x01\0\0\0";
const ALLOWED_WASI_IMPORTS: &[&str] = &["fd_read", "fd_write", "fd_fdstat_get", "proc_exit"];

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
        return Err(rejected(
            "module size, wasm magic, or core version rejected",
        ));
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
                let kind = *bytes
                    .get(pos)
                    .ok_or_else(|| rejected("truncated wasm import"))?;
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
            || self.fuel > MAX_FUEL
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

/// Open a stable inode without following a final symlink. Privileged operator
/// installation is separate: a source belonging to another non-root UID,
/// a writable group/world source, or extra hard links is not trusted.
fn open_engine_source(path: &Path) -> Result<File, ToolError> {
    if !path.is_absolute() {
        return Err(rejected("WASI engine path must be absolute"));
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| rejected("pinned WASI engine is unavailable"))?;
    let meta = file
        .metadata()
        .map_err(|_| rejected("WASI engine metadata unavailable"))?;
    let euid = unsafe { libc::geteuid() };
    if !meta.is_file()
        || meta.nlink() != 1
        || (meta.uid() != euid && meta.uid() != 0)
        || meta.permissions().mode() & 0o111 == 0
        || meta.permissions().mode() & 0o022 != 0
        || meta.len() == 0
        || meta.len() > MAX_ENGINE_BYTES
    {
        return Err(rejected(
            "WASI engine inode ownership, link count, mode or size rejected",
        ));
    }
    Ok(file)
}

fn hash_file(path: &Path) -> Result<[u8; 32], ToolError> {
    let mut file = open_engine_source(path)?;
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

/// Seal an executable snapshot in a Linux anonymous memfd, then execute
/// THAT inode through /proc/self/fd/N instead of reopening the mutable path.
/// A digest verified against the exact copied bytes prevents a path swap
/// between hash_file and execve. F_SEAL_WRITE/SHRINK/GROW/SEAL protects the
/// already-verified executable from in-place mutation.
fn sealed_engine_snapshot(path: &Path, expected: [u8; 32]) -> Result<File, ToolError> {
    let mut source = open_engine_source(path)?;
    let fd = unsafe {
        libc::memfd_create(
            c"ocaf-pinned-wasmtime".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    };
    if fd < 0 {
        return Err(rejected("Linux sealed WASI engine snapshot unavailable"));
    }
    let mut sealed = unsafe { File::from_raw_fd(fd) };
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buf = [0u8; 16_384];
    loop {
        let n = source
            .read(&mut buf)
            .map_err(|_| rejected("cannot read pinned WASI engine"))?;
        if n == 0 {
            break;
        }
        total = total
            .checked_add(n as u64)
            .filter(|bytes| *bytes <= MAX_ENGINE_BYTES)
            .ok_or_else(|| rejected("WASI engine exceeds snapshot limit"))?;
        hasher.update(&buf[..n]);
        sealed
            .write_all(&buf[..n])
            .map_err(|_| rejected("cannot snapshot pinned WASI engine"))?;
    }
    if total == 0 || <[u8; 32]>::from(hasher.finalize()) != expected {
        return Err(rejected("WASI engine snapshot digest mismatch"));
    }
    let seals = libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
    if unsafe { libc::fcntl(sealed.as_raw_fd(), libc::F_ADD_SEALS, seals) } != 0 {
        return Err(rejected("cannot seal pinned WASI executable"));
    }
    Ok(sealed)
}

/// Pinned SHA-256 and an immutable, memory-backed executable inode.
/// The operator must still protect deployment keys and host privileges.
#[derive(Clone)]
pub struct PinnedWasmtime {
    executable: PathBuf,
    sha256: [u8; 32],
    sealed: Arc<File>,
}

impl PinnedWasmtime {
    pub fn new(executable: PathBuf, expected_sha256: [u8; 32]) -> Result<Self, ToolError> {
        let sealed = Arc::new(sealed_engine_snapshot(&executable, expected_sha256)?);
        let engine = Self {
            executable,
            sha256: expected_sha256,
            sealed,
        };
        engine.verify()?;
        Ok(engine)
    }

    fn verify(&self) -> Result<(), ToolError> {
        // Also detect changes to the operator's original installation.
        // Execution nevertheless uses the immutable sealed snapshot.
        if hash_file(&self.executable)? != self.sha256 {
            return Err(rejected("WASI engine digest mismatch"));
        }
        Ok(())
    }
}

/// Terminate an entire subprocess group if execution is cancelled, times out,
/// traps, or returns. Tokio's kill_on_drop covers only the immediate child;
/// a compromised engine could otherwise leave its descendants running.
///
/// This is best-effort Linux process supervision, NOT host sandbox isolation:
/// an adversarial process able to create a new session can escape this group.
struct WasiProcessGroup {
    pgid: libc::pid_t,
}

impl WasiProcessGroup {
    fn for_child(child: &tokio::process::Child) -> Result<Self, ToolError> {
        let pid = child
            .id()
            .and_then(|pid| libc::pid_t::try_from(pid).ok())
            .filter(|pid| *pid > 0)
            .ok_or_else(|| ToolError::Execution("WASI child process ID unavailable".into()))?;
        Ok(Self { pgid: pid })
    }
}

impl Drop for WasiProcessGroup {
    fn drop(&mut self) {
        // Negative PID addresses the dedicated process group (never OCAF's).
        // ESRCH is harmless when the group has already exited. Processes
        // that escape to another group/session require stronger OS isolation.
        unsafe {
            libc::kill(-self.pgid, libc::SIGKILL);
        }
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
        let owner = Self {
            directory,
            module_path,
        };
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
    signed_binding: Option<SignedWasiManifest>,
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
            signed_binding: None,
            engine,
            policy,
        })
    }

    /// Construct a publisher-approved executable whose tool, subject,
    /// immutable module SHA and policy are all part of an Ed25519 signature.
    /// The supplied trust anchor must be installed by an operator.
    pub fn from_signed_manifest(
        engine: PinnedWasmtime,
        module: Vec<u8>,
        signed_json: &[u8],
        signature: &[u8],
        trusted_publisher_key: &[u8],
    ) -> Result<Self, ToolError> {
        let module_hash = sha256(&module);
        let manifest =
            verify_signed_manifest(signed_json, signature, trusted_publisher_key, module_hash)?;
        let policy = manifest.policy.clone();
        let mut executor = Self::new(engine, module, module_hash, policy)?;
        executor.signed_binding = Some(manifest);
        Ok(executor)
    }

    /// Produce an authenticated evidence receipt for a successful execution.
    /// Sign with an operator-managed key outside the untrusted WASI guest.
    /// For rejected requests the caller's audit/replay ledger is authoritative.
    pub async fn execute_with_receipt(
        &self,
        args: Value,
        ctx: &ExecutionContext,
        signer: &ReceiptSigner,
    ) -> Result<(Value, SignedExecutionReceipt), ToolError> {
        let approval = self
            .signed_binding
            .as_ref()
            .ok_or_else(|| rejected("receipts require a signed WASI manifest"))?;
        let request_bytes =
            serde_json::to_vec(&args).map_err(|_| rejected("invalid input for receipt"))?;
        let started = Instant::now();
        let output = self.run(args, ctx).await?;
        let output_bytes =
            serde_json::to_vec(&output).map_err(|_| rejected("invalid output for receipt"))?;
        let completed_unix_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| rejected("receipt clock unavailable"))?
            .as_secs();
        let receipt = ExecutionReceipt {
            schema_version: 1,
            tenant_id: approval.tenant_id.clone(),
            agent_id: approval.agent_id.clone(),
            tool_name: approval.tool.name.clone(),
            correlation_id: ctx.correlation_id.clone(),
            module_sha256_hex: approval.module_sha256_hex.clone(),
            input_sha256_hex: format!("{:x}", Sha256::digest(&request_bytes)),
            output_sha256_hex: format!("{:x}", Sha256::digest(&output_bytes)),
            elapsed_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            completed_unix_secs,
        };
        Ok((output, signer.sign(receipt)?))
    }

    /// Only a signed instance can advertise its operator-approved
    /// ToolManifest to a tool registry.
    pub fn signed_tool_manifest(&self) -> Option<&crate::registry::ToolManifest> {
        self.signed_binding.as_ref().map(|m| &m.tool)
    }

    async fn run(&self, args: Value, ctx: &ExecutionContext) -> Result<Value, ToolError> {
        let _permit = self
            .permits
            .try_acquire()
            .map_err(|_| rejected("WASI sandbox capacity exhausted"))?;
        if ctx.deadline_ms == 0 {
            return Err(rejected("deadline is required"));
        }
        if let Some(approval) = &self.signed_binding {
            // Only the trusted RPC ingress is permitted to supply the
            // authenticated "tenant:agent" principal in ExecutionContext.
            if ctx.agent_id != approval.principal() {
                return Err(rejected("WASI signed manifest principal mismatch"));
            }
            approval.validate(self.approved_sha256)?;
        }
        let mut payload =
            serde_json::to_vec(&args).map_err(|_| rejected("WASI input is not serializable"))?;
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

        let engine_fd = self.engine.sealed.as_raw_fd();
        let mut command = Command::new(format!("/proc/self/fd/{engine_fd}"));
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
        // Isolate the guest's OS process group from the coordinator so
        // cleanup can terminate descendants in addition to the Wasmtime PID.
        command.process_group(0);
        // The only operations in pre_exec are raw Linux syscalls. Failure
        // aborts child spawn; never silently drop the hardening policy.
        unsafe {
            command.as_std_mut().pre_exec(move || {
                // MFD_CLOEXEC protects the coordinator. Clear it only in the
                // forked guest before execve, so binfmt_script can reopen a
                // shebang fixture through /proc/self/fd without mutable path
                // lookup. The parent descriptor stays CLOEXEC.
                let flags = libc::fcntl(engine_fd, libc::F_GETFD);
                if flags < 0 || libc::fcntl(engine_fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                let no_core = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                if libc::setrlimit(libc::RLIMIT_CORE, &no_core) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        let mut child = command
            .spawn()
            .map_err(|_| ToolError::Execution("cannot start pinned WASI runtime".into()))?;
        // Declared after child so Drop sends SIGKILL to the group before
        // Tokio drops the immediate child when a future is cancelled.
        let _group_guard = WasiProcessGroup::for_child(&child)?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| ToolError::Execution("WASI stdin unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ToolError::Execution("WASI stdout unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ToolError::Execution("WASI stderr unavailable".into()))?;

        let limit = self.policy.max_output_bytes;
        let timeout = Duration::from_millis(self.policy.timeout_ms.min(ctx.deadline_ms));
        let completed = tokio::time::timeout(timeout, async {
            let send = async move {
                stdin
                    .write_all(&payload)
                    .await
                    .map_err(|_| ToolError::Execution("WASI stdin write failed".into()))?;
                // WASI callers may wait for EOF; poll_shutdown alone does not
                // guarantee the ChildStdin handle is closed.
                drop(stdin);
                Ok::<(), ToolError>(())
            };
            let output = read_bounded(stdout, limit);
            let error_output = read_bounded(stderr, limit);
            let exit = async {
                child
                    .wait()
                    .await
                    .map_err(|_| ToolError::Execution("WASI process wait failed".into()))
            };
            tokio::try_join!(send, output, error_output, exit)
        })
        .await;

        let completed = match completed {
            Ok(Ok(values)) => values,
            Ok(Err(error)) => {
                let _ = child.start_kill();
                let _ = tokio::time::timeout(Duration::from_millis(500), child.wait()).await;
                return Err(error);
            }
            Err(_) => {
                let _ = child.start_kill();
                let _ = tokio::time::timeout(Duration::from_millis(500), child.wait()).await;
                return Err(ToolError::Timeout);
            }
        };

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
        let size = reader
            .read(&mut buffer)
            .await
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

#[cfg(test)]
mod pinned_engine_tests {
    use super::*;

    #[test]
    fn verified_engine_executes_from_immutable_sealed_memfd() {
        let temp = tempfile::tempdir().expect("temp");
        let path = temp.path().join("operator-engine");
        let trusted = b"#!/bin/sh\\nprintf '{}\\n'\\n";
        fs::write(&path, trusted).expect("create operator executable");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("approve executable");
        let engine = PinnedWasmtime::new(path.clone(), sha256(trusted))
            .expect("snapshot from approved engine");
        let fd = engine.sealed.as_raw_fd();
        let seals = unsafe { libc::fcntl(fd, libc::F_GET_SEALS) };
        let required =
            libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
        assert_eq!(seals & required, required);
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        assert_ne!(
            flags & libc::FD_CLOEXEC,
            0,
            "parent must not leak descriptor"
        );
        let snapshot_path = format!("/proc/self/fd/{fd}");
        assert_eq!(
            fs::read(&snapshot_path).expect("read sealed snapshot"),
            trusted
        );
        assert!(fs::write(&snapshot_path, b"evil replacement").is_err());

        fs::write(&path, b"#!/bin/sh\\nexit 7\\n").expect("mutate origin");
        assert!(engine.verify().is_err(), "changed source must be denied");
        assert_eq!(
            fs::read(&snapshot_path).expect("snapshot survives source mutation"),
            trusted
        );
    }
}
