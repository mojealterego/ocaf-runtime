//! 0.39.7 Aya: real Linux cgroup_skb/egress loader.
//! This intentionally does not bootstrap Firecracker or grant production GO.
//! An actual successful Aya load/attach is necessary, not source presence.
use aya::{
    programs::{CgroupAttachMode, CgroupSkb, CgroupSkbAttachType},
    Ebpf,
};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{self, Error as IoError, ErrorKind, Read},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Component, Path},
    sync::mpsc,
    thread,
    time::Duration,
};

const MAX_PROGRAM_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ATTACH_SECONDS: u64 = 3600;

fn wait_for_supervisor_release<R: Read + Send + 'static>(
    mut reader: R,
    duration: Duration,
) -> bool {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut token = [0_u8; 1];
        let mut extra = [0_u8; 1];
        let accepted = reader.read_exact(&mut token).is_ok()
            && token[0] == b'R'
            && matches!(reader.read(&mut extra), Ok(0));
        let _ = tx.send(accepted);
    });
    matches!(rx.recv_timeout(duration), Ok(true))
}

fn invalid(reason: &'static str) -> IoError {
    IoError::new(ErrorKind::InvalidInput, reason)
}

fn permitted_cgroup(path: &Path) -> bool {
    let prefix = Path::new("/sys/fs/cgroup");
    if !path.is_absolute() || !path.starts_with(prefix) {
        return false;
    }
    if path
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return false;
    }
    let service = path.file_name().and_then(|x| x.to_str()).unwrap_or("");
    service.starts_with("ocaf-")
        && service.ends_with(".service")
        && service.len() <= 110
        && service
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn valid_digest(sha: &str) -> bool {
    sha.len() == 64
        && sha
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn verified_program_bytes(path: &Path, expected_sha256: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(invalid("eBPF program must have an absolute nontraversing path").into());
    }
    if !valid_digest(expected_sha256) {
        return Err(invalid("invalid pinned eBPF digest").into());
    }
    // O_NOFOLLOW and fstat use the SAME descriptor as the hashed bytes.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let st = file.metadata()?;
    if !st.is_file()
        || st.nlink() != 1
        || st.len() == 0
        || st.len() > MAX_PROGRAM_BYTES
        || st.mode() & 0o022 != 0
    {
        return Err(invalid("unsafe eBPF object inode/permissions").into());
    }
    let mut data = Vec::new();
    file.take(MAX_PROGRAM_BYTES + 1).read_to_end(&mut data)?;
    if data.len() != st.len() as usize {
        return Err(invalid("inconsistent eBPF object length").into());
    }
    let actual = format!("{:x}", Sha256::digest(&data));
    if actual != expected_sha256 {
        return Err(invalid("pinned eBPF SHA-256 mismatch").into());
    }
    Ok(data)
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if !matches!(args.len(), 10 | 11)
        || args[1] != "--program"
        || args[3] != "--sha256"
        || args[5] != "--cgroup"
        || args[7] != "--seconds"
        || args[9] != "--ack-attach"
    {
        return Err(invalid(
            "usage: ocaf-stage039-aya-guard --program ABS_ELF --sha256 HEX \\
--cgroup /sys/fs/cgroup/.../ocaf-NAME.service --seconds 1..3600 --ack-attach",
        )
        .into());
    }
    let hold_stdin = args.len() == 11 && args[10] == "--hold-stdin";
    if args.len() == 11 && !hold_stdin {
        return Err(invalid("unknown Aya guard mode").into());
    }
    let duration: u64 = args[8].parse()?;
    if duration == 0 || duration > MAX_ATTACH_SECONDS {
        return Err(invalid("attach lifetime outside bounded range").into());
    }
    let cgroup_path = Path::new(&args[6]);
    if !permitted_cgroup(cgroup_path) {
        return Err(invalid("cgroup must identify an OCAF-specific service").into());
    }
    if fs::symlink_metadata(cgroup_path)?.file_type().is_symlink() {
        return Err(invalid("cgroup may not be a symlink").into());
    }
    let cg = fs::canonicalize(cgroup_path)?;
    if !permitted_cgroup(&cg) || cg != cgroup_path {
        return Err(invalid("cgroup path canonicalization mismatch").into());
    }
    let cg_meta = fs::metadata(&cg)?;
    if !cg_meta.is_dir() {
        return Err(invalid("target cgroup is not a directory").into());
    }
    let bpf = verified_program_bytes(Path::new(&args[2]), &args[4])?;
    let mut obj = Ebpf::load(&bpf)?;
    let program: &mut CgroupSkb = obj
        .program_mut("ocaf_deny_egress")
        .ok_or_else(|| invalid("named egress program absent"))?
        .try_into()?;
    program.load()?;
    // Genuine BPF kernel load does not mean attachment. Only an attach
    // return value proves acceptance of the cgroup hook by the kernel.
    let link_id = program.attach(
        File::open(&cg)?,
        CgroupSkbAttachType::Egress,
        CgroupAttachMode::Single,
    )?;
    let program_id = program.info()?.id();
    println!(
        "{{\"status\":\"AYA_CGROUP_EGRESS_ATTACHED\",\"program_id\":{},\"cgroup_inode\":{},\"guest_booted\":false,\"production_approved\":false}}",
        program_id, cg_meta.ino()
    );
    // In lifecycle mode the supervisor releases the link after it has
    // stopped the VM. An unresponsive supervisor cannot hold it forever.
    let release_ok = if hold_stdin {
        wait_for_supervisor_release(io::stdin(), Duration::from_secs(duration))
    } else {
        thread::sleep(Duration::from_secs(duration));
        true
    };
    program.detach(link_id)?;
    println!(
        "{{\"status\":\"AYA_CGROUP_EGRESS_DETACHED\",\"program_id\":{},\"production_approved\":false}}",
        program_id
    );
    if !release_ok {
        return Err(invalid("Aya supervisor release missing or deadline expired").into());
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("STAGE039_AYA_DENIED: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn supervisor_release_requires_exact_single_token_and_eof() {
        for (input, expected) in [
            (b"R".as_slice(), true),
            (b"X".as_slice(), false),
            (b"RR".as_slice(), false),
            (b"".as_slice(), false),
        ] {
            assert_eq!(
                wait_for_supervisor_release(Cursor::new(input.to_vec()), Duration::from_secs(1)),
                expected
            );
        }
    }
    #[test]
    fn only_ocaf_service_cgroup_is_allowed() {
        assert!(permitted_cgroup(Path::new(
            "/sys/fs/cgroup/system.slice/ocaf-vm-a.service"
        )));
        assert!(!permitted_cgroup(Path::new("/sys/fs/cgroup")));
        assert!(!permitted_cgroup(Path::new(
            "/sys/fs/cgroup/system.slice/sshd.service"
        )));
        assert!(!permitted_cgroup(Path::new("/etc/ocaf-vm-a.service")));
        assert!(!permitted_cgroup(Path::new(
            "/sys/fs/cgroup/ocaf-a.service/../sshd.service"
        )));
    }
    #[test]
    fn digest_must_be_lower_hex_exact() {
        assert!(valid_digest(&"a".repeat(64)));
        assert!(!valid_digest(&"A".repeat(64)));
        assert!(!valid_digest(&"a".repeat(63)));
        assert!(!valid_digest(&"x".repeat(64)));
    }
    #[test]
    fn rejects_missing_elf_without_loading_kernel_program() {
        let result = verified_program_bytes(
            Path::new("/nonexistent/ocaf-stage039-ebpf.o"),
            &"0".repeat(64),
        );
        assert!(result.is_err());
    }
}
