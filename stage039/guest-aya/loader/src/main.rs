//! Guest Aya openat probe. PID1 holds the Wasmtime child behind a pipe until
//! this process prints READY, then requires EVENT before accepting the task.
use aya::{Ebpf, Pod, maps::Array, programs::TracePoint};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    io::{self, Write},
    thread,
    time::{Duration, Instant},
};

// The build script sets these to the SHA-pinned result of compiling the Aya
// eBPF crate. The ELF is embedded so PID1 retains its two-argument ABI.
const EBPF_ELF: &[u8] = include_bytes!(env!("OCAF_GUEST_AYA_EBPF_PATH"));
const EBPF_SHA256: &str = env!("OCAF_GUEST_AYA_EBPF_SHA256");
const MAX_TIMEOUT: u64 = 120;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct OpenatCount {
    target_tgid: u32,
    reserved: u32,
    count: u64,
    pid_tgid: u64,
}

unsafe impl Pod for OpenatCount {}

fn parse_args(args: &[String]) -> Result<(u32, u64, u64), &'static str> {
    if args.len() != 3 && args.len() != 4 {
        return Err("target TGID, timeout and optional minimum events required");
    }
    let tgid: u32 = args[1].parse().map_err(|_| "invalid target TGID")?;
    let seconds: u64 = args[2].parse().map_err(|_| "invalid timeout")?;
    let minimum: u64 = if args.len() == 4 {
        args[3].parse().map_err(|_| "invalid minimum events")?
    } else { 1 };
    if tgid <= 1 || seconds == 0 || seconds > MAX_TIMEOUT || minimum == 0 || minimum > 16 {
        return Err("target, timeout or minimum events outside bounds");
    }
    Ok((tgid, seconds, minimum))
}

fn assert_embedded_digest() -> Result<(), &'static str> {
    if EBPF_SHA256.len() != 64
        || !EBPF_SHA256
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        || format!("{:x}", Sha256::digest(EBPF_ELF)) != EBPF_SHA256
    {
        return Err("embedded Aya program digest mismatch");
    }
    Ok(())
}

fn mount_guest_tracefs() -> Result<(), Box<dyn Error>> {
    // The guest image creates these directories before making ext4 read-only.
    // A denied mount is acceptable only if a tracepoint ID was already there.
    let paths = [
        "/sys/kernel/tracing/events/syscalls/sys_enter_openat/id",
        "/sys/kernel/debug/tracing/events/syscalls/sys_enter_openat/id",
    ];
    if paths.iter().any(|p| std::path::Path::new(p).is_file()) {
        return Ok(());
    }
    for (source, target, kind) in [
        ("sysfs", "/sys", "sysfs"),
        ("tracefs", "/sys/kernel/tracing", "tracefs"),
        ("debugfs", "/sys/kernel/debug", "debugfs"),
    ] {
        let src = std::ffi::CString::new(source)?;
        let dst = std::ffi::CString::new(target)?;
        let typ = std::ffi::CString::new(kind)?;
        unsafe {
            libc::mount(
                src.as_ptr(),
                dst.as_ptr(),
                typ.as_ptr(),
                (libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC) as libc::c_ulong,
                std::ptr::null(),
            );
        }
        if paths.iter().any(|p| std::path::Path::new(p).is_file()) {
            return Ok(());
        }
    }
    Err("guest sys_enter_openat tracepoint unavailable".into())
}

fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let (target_tgid, seconds, minimum) = parse_args(args)?;
    assert_embedded_digest()?;
    mount_guest_tracefs()?;
    let mut bpf = Ebpf::load(EBPF_ELF)?;
    {
        let mut counts: Array<_, OpenatCount> = Array::try_from(
            bpf.map_mut("OPENAT_COUNTS")
                .ok_or("Aya count map missing")?,
        )?;
        counts.set(
            0,
            OpenatCount {
                target_tgid,
                ..Default::default()
            },
            0,
        )?;
    }
    let program: &mut TracePoint = bpf
        .program_mut("ocaf_guest_openat")
        .ok_or("Aya openat program missing")?
        .try_into()?;
    program.load()?;
    let id = program.info()?.id();
    if id == 0 {
        return Err("guest kernel returned zero Aya program ID".into());
    }
    let _link = program.attach("syscalls", "sys_enter_openat")?;
    println!("OCAF039_GUEST_OPENAT_PROBE_READY_ID={id}");
    io::stdout().flush()?;
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        let record = {
            let counts: Array<_, OpenatCount> =
                Array::try_from(bpf.map("OPENAT_COUNTS").ok_or("Aya count map lost")?)?;
            counts.get(&0, 0)?
        };
        if record.count >= minimum && (record.pid_tgid >> 32) as u32 == target_tgid {
            let pid = record.pid_tgid as u32;
            if pid == 0 {
                return Err("guest Aya observed zero PID".into());
            }
            println!(
                "OCAF039_GUEST_OPENAT_EVENT_ID={id} COUNT={} TGID={target_tgid} PID={pid}",
                record.count
            );
            io::stdout().flush()?;
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("guest Aya openat event deadline expired".into());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(err) = run(&args) {
        eprintln!("OCAF039_GUEST_OPENAT_AYA_DENIED={err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_ambiguous_target_and_unbounded_timeout() {
        for args in [
            vec!["probe".to_string()],
            vec!["probe".into(), "0".into(), "30".into()],
            vec!["probe".into(), "42".into(), "0".into()],
            vec!["probe".into(), "42".into(), "121".into()],
            vec!["probe".into(), "42x".into(), "30".into()],
        ] {
            assert!(parse_args(&args).is_err());
        }
        assert_eq!(
            parse_args(&["probe".into(), "42".into(), "30".into()]),
            Ok((42, 30, 1))
        );
        assert_eq!(
            parse_args(&["probe".into(), "42".into(), "30".into(), "2".into()]),
            Ok((42, 30, 2))
        );
        assert!(parse_args(&["probe".into(), "42".into(), "30".into(), "17".into()]).is_err());
    }
    #[test]
    fn embedded_program_matches_pinned_build_digest() {
        assert_embedded_digest().unwrap();
        let object = aya_obj::Object::parse(EBPF_ELF).unwrap();
        assert!(object.programs.contains_key("ocaf_guest_openat"));
        assert!(object.maps.contains_key("OPENAT_COUNTS"));
    }
}
